use super::fixture::*;
use std::collections::BTreeSet;
use std::sync::{Arc, Barrier};
use ultragoal::orchestration::product::runtime_adapter::{
    OrchestrationRuntimeAdapter, RuntimeActionRequest, RuntimeActionSource,
};
use ultragoal::orchestration::product::{
    PermitReplayState, ProductError, ProductWorkspace, ProductionRootAuthority, RecoverRequest,
};

#[test]
fn interrupted_event_binding_tick_workers_and_target_substitutions_do_not_reserve() {
    let (journal, prior, _, inspection) = interrupted_heartbeat("recover-binding");
    let authority_root = TestRoot::new("recover-binding-authority", 0o700);
    let initial_context = context();
    let workspace = ProductWorkspace::open(journal.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&initial_context, &workspace).unwrap();
    let view = adapter.inspect_interrupted(&inspection).unwrap();
    let action = view.preview().root_action_request.clone();
    let authority =
        ProductionRootAuthority::open_or_initialize(authority_root.path(), root_actor()).unwrap();
    let permit = adapter
        .issue_production_action(
            &authority,
            RuntimeActionSource::Interrupted(&view),
            &action,
            inspection.tick + 10,
        )
        .unwrap();
    let exact = RecoverRequest {
        expected_prior_head: prior,
        expected_event_id: inspection.expected_event_id.clone(),
        recovered_binding: inspection.recovered_binding.clone(),
        tick: inspection.tick,
        live_workers: inspection.live_workers.clone(),
        target: action.target.clone(),
    };
    let journal_before = recursive_fingerprint(journal.path());
    let authority_before = recursive_fingerprint(authority_root.path());

    for changed in substitutions(&exact) {
        assert_eq!(
            adapter
                .execute_production_action(
                    &authority,
                    RuntimeActionSource::Interrupted(&view),
                    &action,
                    &permit,
                    &RuntimeActionRequest::Recover(changed),
                )
                .unwrap_err(),
            ProductError::AuthorityOperationMismatch
        );
        assert_eq!(recursive_fingerprint(journal.path()), journal_before);
        assert_eq!(
            recursive_fingerprint(authority_root.path()),
            authority_before
        );
        assert_eq!(
            authority.replay_state(&permit).unwrap(),
            Some(PermitReplayState::Issued)
        );
    }
}

#[test]
fn concurrent_recovery_has_one_production_authority_winner() {
    let (journal, prior, _, inspection) = interrupted_heartbeat("recover-race");
    let authority_root = TestRoot::new("recover-race-authority", 0o700);
    let initial_context = context();
    let workspace = ProductWorkspace::open(journal.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&initial_context, &workspace).unwrap();
    let view = adapter.inspect_interrupted(&inspection).unwrap();
    let action = view.preview().root_action_request.clone();
    let authority = Arc::new(
        ProductionRootAuthority::open_or_initialize(authority_root.path(), root_actor()).unwrap(),
    );
    let permit = adapter
        .issue_production_action(
            &authority,
            RuntimeActionSource::Interrupted(&view),
            &action,
            inspection.tick + 10,
        )
        .unwrap();
    let request = RecoverRequest {
        expected_prior_head: prior,
        expected_event_id: inspection.expected_event_id.clone(),
        recovered_binding: inspection.recovered_binding.clone(),
        tick: inspection.tick,
        live_workers: inspection.live_workers.clone(),
        target: action.target.clone(),
    };
    let barrier = Arc::new(Barrier::new(2));
    let journal_path = journal.path().to_owned();
    let outcomes = std::thread::scope(|scope| {
        (0..2)
            .map(|_| {
                let barrier = Arc::clone(&barrier);
                let authority = Arc::clone(&authority);
                let permit = permit.clone();
                let action = action.clone();
                let request = request.clone();
                let inspection = inspection.clone();
                let journal_path = journal_path.clone();
                scope.spawn(move || {
                    let context = context();
                    let workspace = ProductWorkspace::open(&journal_path).unwrap();
                    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
                    barrier.wait();
                    let view = adapter.inspect_interrupted(&inspection)?;
                    adapter.execute_production_action(
                        &authority,
                        RuntimeActionSource::Interrupted(&view),
                        &action,
                        &permit,
                        &RuntimeActionRequest::Recover(request),
                    )
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|child| child.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        authority.replay_state(&permit).unwrap(),
        Some(PermitReplayState::Committed)
    );
}

fn substitutions(exact: &RecoverRequest) -> Vec<RecoverRequest> {
    let mut event = exact.clone();
    event.expected_event_id = digest('f');
    let mut binding = exact.clone();
    binding.recovered_binding =
        ultragoal::orchestration::Binding::new(&digest('c'), &digest('d')).unwrap();
    let mut tick = exact.clone();
    tick.tick += 1;
    let mut workers = exact.clone();
    workers.live_workers = BTreeSet::new();
    let mut target = exact.clone();
    target.target.operation_id = Some(digest('f'));
    vec![event, binding, tick, workers, target]
}
