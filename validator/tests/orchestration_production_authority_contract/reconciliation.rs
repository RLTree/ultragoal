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

#[test]
fn reconciliation_only_and_no_pending_views_cannot_issue_the_wrong_authority() {
    let (ambiguous_journal, ambiguous_head) = ambiguous_effect("reconcile-only-view");
    let authority_root = TestRoot::new("reconcile-only-authority", 0o700);
    let context = context();
    let ambiguous_workspace = ProductWorkspace::open(ambiguous_journal.path()).unwrap();
    let ambiguous_adapter =
        OrchestrationRuntimeAdapter::new(&context, &ambiguous_workspace).unwrap();
    let ambiguous_view = ambiguous_adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: ambiguous_head,
            tick: 4,
            live_workers: BTreeSet::from(["worker-a".to_owned()]),
        })
        .unwrap();
    let reconcile_action = ambiguous_view.state().root_action_requests[0].clone();
    let authority =
        ProductionRootAuthority::open_or_initialize(authority_root.path(), root_actor()).unwrap();
    let ambiguous_before = recursive_fingerprint(ambiguous_journal.path());
    assert_eq!(
        ambiguous_adapter
            .issue_production_action(
                &authority,
                RuntimeActionSource::Current(&ambiguous_view),
                &reconcile_action,
                14,
            )
            .unwrap_err(),
        ProductError::AuthorityOperationMismatch
    );
    assert_eq!(
        recursive_fingerprint(ambiguous_journal.path()),
        ambiguous_before
    );

    let (clean_journal, clean_head) = running_lease("no-pending-reconciliation", 40);
    let clean_workspace = ProductWorkspace::open(clean_journal.path()).unwrap();
    let clean_adapter = OrchestrationRuntimeAdapter::new(&context, &clean_workspace).unwrap();
    let clean_view = clean_adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: clean_head,
            tick: 3,
            live_workers: BTreeSet::from(["worker-a".to_owned()]),
        })
        .unwrap();
    assert!(clean_view.state().root_action_requests.is_empty());
    let clean_before = recursive_fingerprint(clean_journal.path());
    assert_eq!(
        clean_adapter
            .issue_production_reconcile(
                &authority,
                &clean_view,
                &reconcile_action,
                13,
                &not_applied_resolution(),
            )
            .unwrap_err(),
        ProductError::AuthorityInvalid
    );
    assert_eq!(recursive_fingerprint(clean_journal.path()), clean_before);
}
