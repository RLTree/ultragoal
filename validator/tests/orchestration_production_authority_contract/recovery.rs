use super::fixture::*;
use ultragoal::orchestration::product::runtime_adapter::{
    OrchestrationRuntimeAdapter, RuntimeActionOutcome, RuntimeActionRequest, RuntimeActionSource,
};
use ultragoal::orchestration::product::{
    PermitReplayState, ProductWorkspace, ProductionRootAuthority, RecoverRequest,
};

#[test]
fn interrupted_publication_uses_production_issuance_and_blocks_fresh_reopen() {
    let (journal, prior, event, request) = interrupted_heartbeat("recover-journal");
    let authority_root = TestRoot::new("recover-authority", 0o700);
    let context = context();
    let workspace = ProductWorkspace::open(journal.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let view = adapter.inspect_interrupted(&request).unwrap();
    let action = view.preview().root_action_request.clone();
    let authority =
        ProductionRootAuthority::open_or_initialize(authority_root.path(), root_actor()).unwrap();
    let permit = adapter
        .issue_production_action(
            &authority,
            RuntimeActionSource::Interrupted(&view),
            &action,
            request.tick + 10,
        )
        .unwrap();
    let outcome = adapter
        .execute_production_action(
            &authority,
            RuntimeActionSource::Interrupted(&view),
            &action,
            &permit,
            &RuntimeActionRequest::Recover(RecoverRequest {
                expected_prior_head: prior,
                expected_event_id: request.expected_event_id,
                recovered_binding: request.recovered_binding,
                tick: request.tick,
                live_workers: request.live_workers,
                target: action.target.clone(),
            }),
        )
        .unwrap();
    let RuntimeActionOutcome::Recover(recovered) = outcome else {
        panic!("recover outcome required");
    };
    assert_eq!(recovered.recovered_head.last_event_id, event.event_id);
    assert_eq!(
        authority.replay_state(&permit).unwrap(),
        Some(PermitReplayState::Committed)
    );
    drop(authority);
    assert_eq!(
        ProductionRootAuthority::open_existing(authority_root.path(), root_actor()).unwrap_err(),
        ultragoal::orchestration::product::ProductError::AuthorityCheckpointRequired
    );
}
