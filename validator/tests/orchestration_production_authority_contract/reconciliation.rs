use super::fixture::*;
use std::collections::BTreeSet;
use ultragoal::orchestration::product::command::OrchestrationStateRequest;
use ultragoal::orchestration::product::runtime_adapter::OrchestrationRuntimeAdapter;
use ultragoal::orchestration::product::runtime_adapter::{
    RuntimeActionRequest, RuntimeActionSource,
};
use ultragoal::orchestration::product::{
    PermitReplayState, ProductError, ProductWorkspace, ProductionRootAuthority, ReconcileRequest,
    ResumeRequest,
};

#[test]
fn decision_bound_reconciliation_is_issued_consumed_and_fresh_reopen_is_blocked() {
    let (journal, head) = ambiguous_effect("reconcile-journal");
    let authority_root = TestRoot::new("reconcile-authority", 0o700);
    let context = context();
    let workspace = ProductWorkspace::open(journal.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let view = adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: head.clone(),
            tick: 4,
            live_workers: BTreeSet::from(["worker-a".to_owned()]),
        })
        .unwrap();
    let action = view.state().root_action_requests[0].clone();
    let resolution = not_applied_resolution();
    let authority =
        ProductionRootAuthority::open_or_initialize(authority_root.path(), root_actor()).unwrap();
    let permit = adapter
        .issue_production_reconcile(&authority, &view, &action, 14, &resolution)
        .unwrap();
    assert_eq!(
        authority.replay_state(&permit).unwrap(),
        Some(PermitReplayState::Issued)
    );
    let outcome = adapter
        .execute_production_reconcile(
            &authority,
            &view,
            &action,
            &permit,
            &ReconcileRequest {
                expected_head: head,
                tick: 4,
                live_workers: BTreeSet::from(["worker-a".to_owned()]),
                lease_id: "lease-001".to_owned(),
                resolution,
                target: action.target.clone(),
            },
        )
        .unwrap();
    assert_eq!(outcome.settled_operation_id, "operation-001");
    assert_eq!(
        authority.replay_state(&permit).unwrap(),
        Some(PermitReplayState::Committed)
    );
    drop(authority);
    assert_eq!(
        ProductionRootAuthority::open_existing(authority_root.path(), root_actor()).unwrap_err(),
        ProductError::AuthorityCheckpointRequired
    );
}

#[test]
fn invalid_request_refuses_before_reservation_and_permit_remains_usable() {
    let (journal, head) = interrupted_root("ambiguous-refusal-journal");
    let authority_root = TestRoot::new("ambiguous-refusal-authority", 0o700);
    let (action, permit, authority) = issue_resume(&authority_root, &journal, head.clone(), 2);
    let context = context();
    let workspace = ProductWorkspace::open(journal.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let view = adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: head.clone(),
            tick: 2,
            live_workers: BTreeSet::new(),
        })
        .unwrap();
    let request = RuntimeActionRequest::Resume(ResumeRequest {
        expected_head: head,
        tick: 3,
        live_workers: BTreeSet::new(),
        target: action.target.clone(),
    });
    assert_eq!(
        adapter
            .execute_production_action(
                &authority,
                RuntimeActionSource::Current(&view),
                &action,
                &permit,
                &request,
            )
            .unwrap_err(),
        ProductError::AuthorityOperationMismatch
    );
    assert_eq!(
        authority.replay_state(&permit).unwrap(),
        Some(PermitReplayState::Issued)
    );
    let valid_request = RuntimeActionRequest::Resume(ResumeRequest {
        expected_head: action.expected_head.clone(),
        tick: 2,
        live_workers: BTreeSet::new(),
        target: action.target.clone(),
    });
    adapter
        .execute_production_action(
            &authority,
            RuntimeActionSource::Current(&view),
            &action,
            &permit,
            &valid_request,
        )
        .unwrap();
    assert_eq!(
        authority.replay_state(&permit).unwrap(),
        Some(PermitReplayState::Committed)
    );
}
