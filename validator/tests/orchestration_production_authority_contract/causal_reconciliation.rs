use super::fixture::*;
use ultragoal::orchestration::product::command::OrchestrationStateRequest;
use ultragoal::orchestration::product::runtime_adapter::{
    OrchestrationRuntimeAdapter, RuntimeActionSource,
};
use ultragoal::orchestration::product::{
    PermitReplayState, ProductError, ProductWorkspace, ProductionRootAuthority,
};

#[test]
fn one_causal_observation_cannot_issue_two_indistinguishable_permits() {
    let (journal, head) = interrupted_root("duplicate-causal-slot-journal");
    let authority_root = TestRoot::new("duplicate-causal-slot-authority", 0o700);
    let context = context();
    let workspace = ProductWorkspace::open(journal.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let view = adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: head,
            tick: 2,
            live_workers: Default::default(),
        })
        .unwrap();
    let action = view.state().root_action_requests[0].clone();
    let authority =
        ProductionRootAuthority::open_or_initialize(authority_root.path(), root_actor()).unwrap();
    let first = adapter
        .issue_production_action(&authority, RuntimeActionSource::Current(&view), &action, 12)
        .unwrap();
    let before = recursive_fingerprint(authority_root.path());
    assert_eq!(
        adapter
            .issue_production_action(&authority, RuntimeActionSource::Current(&view), &action, 13,)
            .unwrap_err(),
        ProductError::AuthorityReplay
    );
    assert_eq!(recursive_fingerprint(authority_root.path()), before);
    assert_eq!(
        authority.replay_state(&first).unwrap(),
        Some(PermitReplayState::Issued)
    );
}

#[test]
fn nonempty_reopen_requires_unimplemented_external_monotonic_custody() {
    let (journal, head) = interrupted_root("checkpoint-blocker-journal");
    let authority_root = TestRoot::new("checkpoint-blocker-authority", 0o700);
    let (_, _, authority) = issue_resume(&authority_root, &journal, head, 2);
    drop(authority);
    let before = recursive_fingerprint(authority_root.path());
    assert_eq!(
        ProductionRootAuthority::open_existing(authority_root.path(), root_actor()).unwrap_err(),
        ProductError::AuthorityCheckpointRequired
    );
    assert_eq!(recursive_fingerprint(authority_root.path()), before);
}
