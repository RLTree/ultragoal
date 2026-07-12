use super::support::*;
use crate::orchestration::product::*;
use crate::orchestration::*;
use crate::runtime_adapter::*;
use std::sync::{Arc, Barrier};

#[test]
fn concurrent_recovery_has_one_authoritative_winner_and_replay_refuses() {
    let (root, prior, _, inspection) = interrupted_heartbeat("race-recover");
    let context = context();
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let view = adapter.inspect_interrupted(&inspection).unwrap();
    let action = view.preview().root_action_request.clone();
    let authority = authority();
    let permit = permit_for_action(&authority, &action, inspection.tick);
    let request = RuntimeActionRequest::Recover(RecoverRequest {
        expected_prior_head: inspection.expected_prior_head.clone(),
        expected_event_id: inspection.expected_event_id.clone(),
        recovered_binding: inspection.recovered_binding.clone(),
        tick: inspection.tick,
        live_workers: inspection.live_workers.clone(),
        target: action.target.clone(),
    });
    let barrier = Arc::new(Barrier::new(2));
    let results = (0..2)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            let context = context.clone();
            let workspace = workspace.clone();
            let view = view.clone();
            let action = action.clone();
            let authority = authority.clone();
            let permit = permit.clone();
            let request = request.clone();
            std::thread::spawn(move || {
                let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
                barrier.wait();
                adapter.execute_action(
                    RuntimeActionSource::Interrupted(&view),
                    &action,
                    &authority,
                    &permit,
                    &request,
                )
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(ProductError::ConcurrentUpdate)))
            .count(),
        1
    );
    let winner = results.into_iter().find_map(Result::ok).unwrap();
    let RuntimeActionOutcome::Recover(winner) = winner else {
        panic!("recovery race returned the wrong outcome")
    };
    let committed = FileJournal::open(root.path()).unwrap().inspect().unwrap();
    assert_eq!(committed.head, winner.recovered_head);
    assert_eq!(committed.head.event_count, prior.event_count + 1);

    let before_replay = recursive_fingerprint(root.path());
    assert_eq!(
        adapter
            .execute_action(
                RuntimeActionSource::Interrupted(&view),
                &action,
                &authority,
                &permit,
                &request,
            )
            .unwrap_err(),
        ProductError::ConcurrentUpdate
    );
    assert_eq!(recursive_fingerprint(root.path()), before_replay);
}

#[test]
fn same_view_differently_bound_reconciliations_have_one_current_head_winner() {
    let (root, _) = ambiguous_effect("race-reconcile");
    let context = context();
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let head = FileJournal::open(root.path())
        .unwrap()
        .inspect()
        .unwrap()
        .head;
    let inspection = state_request(head, 4);
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let view = adapter.inspect_current(&inspection).unwrap();
    let action = view.state().root_action_requests[0].clone();
    let authority = authority();
    let not_applied = ReconcileRequest {
        expected_head: inspection.expected_head.clone(),
        tick: inspection.tick,
        live_workers: inspection.live_workers.clone(),
        lease_id: "lease-001".to_owned(),
        resolution: not_applied_resolution(),
        target: action.target.clone(),
    };
    let applied = ReconcileRequest {
        resolution: applied_resolution(),
        ..not_applied.clone()
    };
    let not_applied_permit = permit_for_reconciliation(
        &authority,
        &action,
        inspection.tick,
        &not_applied.resolution,
    );
    let applied_permit =
        permit_for_reconciliation(&authority, &action, inspection.tick, &applied.resolution);
    let barrier = Arc::new(Barrier::new(2));
    let results = [
        (not_applied_permit.clone(), not_applied.clone()),
        (applied_permit, applied),
    ]
    .into_iter()
    .map(|(permit, request)| {
        let barrier = Arc::clone(&barrier);
        let context = context.clone();
        let workspace = workspace.clone();
        let view = view.clone();
        let action = action.clone();
        let authority = authority.clone();
        std::thread::spawn(move || {
            let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
            barrier.wait();
            adapter.execute_reconcile(&view, &action, &authority, &permit, &request)
        })
    })
    .collect::<Vec<_>>()
    .into_iter()
    .map(|handle| handle.join().unwrap())
    .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(ProductError::ConcurrentUpdate)))
            .count(),
        1
    );
    let before_replay = recursive_fingerprint(root.path());
    assert_eq!(
        adapter
            .execute_reconcile(
                &view,
                &action,
                &authority,
                &not_applied_permit,
                &not_applied,
            )
            .unwrap_err(),
        ProductError::ConcurrentUpdate
    );
    assert_eq!(recursive_fingerprint(root.path()), before_replay);
    let current = FileJournal::open(root.path()).unwrap().inspect().unwrap();
    let reopened = adapter
        .inspect_current(
            &crate::orchestration::product::command::OrchestrationStateRequest {
                expected_head: current.head,
                tick: inspection.tick,
                live_workers: inspection.live_workers,
            },
        )
        .unwrap();
    assert!(
        reopened
            .state()
            .snapshot
            .settled_operations
            .contains("operation-001")
    );
}
