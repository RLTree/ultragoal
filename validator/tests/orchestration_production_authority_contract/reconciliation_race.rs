use super::fixture::*;
use std::collections::BTreeSet;
use std::sync::{Arc, Barrier};
use ultragoal::orchestration::product::command::OrchestrationStateRequest;
use ultragoal::orchestration::product::runtime_adapter::OrchestrationRuntimeAdapter;
use ultragoal::orchestration::product::{
    PermitReplayState, ProductError, ProductWorkspace, ProductionRootAuthority, ReconcileRequest,
};
use ultragoal::orchestration::{EffectOutcome, EffectReceipt, EffectResolution};

#[test]
fn concurrent_differently_bound_reconciliations_have_one_current_head_winner() {
    let (journal, head) = ambiguous_effect("reconcile-current-head-race");
    let first_root = TestRoot::new("reconcile-race-first-authority", 0o700);
    let second_root = TestRoot::new("reconcile-race-second-authority", 0o700);
    let initial_context = context();
    let workspace = ProductWorkspace::open(journal.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&initial_context, &workspace).unwrap();
    let inspection = OrchestrationStateRequest {
        expected_head: head.clone(),
        tick: 4,
        live_workers: BTreeSet::from(["worker-a".to_owned()]),
    };
    let view = adapter.inspect_current(&inspection).unwrap();
    let action = view.state().root_action_requests[0].clone();
    let first =
        ProductionRootAuthority::open_or_initialize(first_root.path(), root_actor()).unwrap();
    let second =
        ProductionRootAuthority::open_or_initialize(second_root.path(), root_actor()).unwrap();
    let first_request = request(&action, head.clone(), not_applied_resolution());
    let second_request = request(&action, head, applied_resolution());
    let first_permit = adapter
        .issue_production_reconcile(&first, &view, &action, 14, &first_request.resolution)
        .unwrap();
    let second_permit = adapter
        .issue_production_reconcile(&second, &view, &action, 14, &second_request.resolution)
        .unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let journal_path = journal.path().to_owned();

    let outcomes = std::thread::scope(|scope| {
        let attempts = [
            (&first, first_permit.clone(), first_request.clone()),
            (&second, second_permit.clone(), second_request.clone()),
        ];
        attempts
            .into_iter()
            .map(|(authority, permit, request)| {
                let barrier = Arc::clone(&barrier);
                let action = action.clone();
                let view = view.clone();
                let journal_path = journal_path.clone();
                scope.spawn(move || {
                    barrier.wait();
                    let context = context();
                    let workspace = ProductWorkspace::open(&journal_path).unwrap();
                    OrchestrationRuntimeAdapter::new(&context, &workspace)
                        .unwrap()
                        .execute_production_reconcile(authority, &view, &action, &permit, &request)
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|child| child.join().unwrap())
            .collect::<Vec<_>>()
    });

    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, Err(ProductError::ConcurrentUpdate)))
            .count(),
        1
    );
    let states = [
        first.replay_state(&first_permit).unwrap().unwrap(),
        second.replay_state(&second_permit).unwrap().unwrap(),
    ];
    assert_eq!(
        states
            .iter()
            .filter(|state| **state == PermitReplayState::Committed)
            .count(),
        1
    );
    assert_eq!(
        states
            .iter()
            .filter(|state| **state == PermitReplayState::Ambiguous)
            .count(),
        1
    );
}

fn request(
    action: &ultragoal::orchestration::product::command::RootActionRequest,
    head: ultragoal::orchestration::JournalHead,
    resolution: EffectResolution,
) -> ReconcileRequest {
    ReconcileRequest {
        expected_head: head,
        tick: 4,
        live_workers: BTreeSet::from(["worker-a".to_owned()]),
        lease_id: "lease-001".to_owned(),
        resolution,
        target: action.target.clone(),
    }
}

fn applied_resolution() -> EffectResolution {
    EffectResolution {
        operation_id: "operation-001".to_owned(),
        evidence_digest: digest('f'),
        outcome: EffectOutcome::Applied {
            receipt: EffectReceipt {
                operation_id: "operation-001".to_owned(),
                effect: effect(),
                receipt_digest: digest('d'),
            },
        },
    }
}
