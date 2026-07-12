use super::support::*;
use crate::orchestration::product::command::{CommandProjection, OrchestrationStateRequest};
use crate::orchestration::product::*;
use crate::orchestration::*;
use crate::runtime_adapter::*;
use std::collections::BTreeSet;

#[test]
fn every_adapter_read_projection_is_recursive_zero_write() {
    let (current_root, current_engine) = durable_engine("read-only-current", false);
    let current_head = current_engine.journal_head().unwrap().clone();
    drop(current_engine);
    let current_context = context();
    let current_workspace = ProductWorkspace::open(current_root.path()).unwrap();
    let current_adapter =
        OrchestrationRuntimeAdapter::new(&current_context, &current_workspace).unwrap();
    let before_current = recursive_fingerprint(current_root.path());
    let current = current_adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: current_head,
            tick: 1,
            live_workers: BTreeSet::new(),
        })
        .unwrap();
    for projection in [
        CommandProjection::Inspect,
        CommandProjection::Next,
        CommandProjection::Diagnose(None),
        CommandProjection::Diagnose(Some(digest('f'))),
    ] {
        let _ = current_adapter
            .project_current(&current, &projection)
            .unwrap();
    }
    assert_eq!(recursive_fingerprint(current_root.path()), before_current);

    let (interrupted_root, _, _, interrupted_request) =
        interrupted_heartbeat("read-only-interrupted");
    let interrupted_context = context();
    let interrupted_workspace = ProductWorkspace::open(interrupted_root.path()).unwrap();
    let interrupted_adapter =
        OrchestrationRuntimeAdapter::new(&interrupted_context, &interrupted_workspace).unwrap();
    let before_interrupted = recursive_fingerprint(interrupted_root.path());
    let _ = interrupted_adapter
        .inspect_interrupted(&interrupted_request)
        .unwrap();
    assert_eq!(
        recursive_fingerprint(interrupted_root.path()),
        before_interrupted
    );
}

#[test]
fn sealed_current_view_projects_without_writes_then_resumes_exact_root_state() {
    let (root, head, commitment) = interrupted_root("positive-resume");
    let context = context();
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let request = state_request(head.clone(), 5);
    let before_read = recursive_fingerprint(root.path());
    let view = adapter.inspect_current(&request).unwrap();
    for projection in [
        CommandProjection::Inspect,
        CommandProjection::Next,
        CommandProjection::Diagnose(None),
    ] {
        assert!(
            adapter
                .project_current(&view, &projection)
                .unwrap()
                .is_some()
        );
    }
    assert_eq!(recursive_fingerprint(root.path()), before_read);

    let action = view.state().root_action_requests[0].clone();
    assert_eq!(action.operation, RootOperation::Resume);
    let authority = authority();
    let permit = permit_for_action(&authority, &action, request.tick);
    let runtime_request = RuntimeActionRequest::Resume(ResumeRequest {
        expected_head: head,
        tick: request.tick,
        live_workers: request.live_workers.clone(),
        target: action.target.clone(),
    });
    let outcome = adapter
        .execute_action(
            RuntimeActionSource::Current(&view),
            &action,
            &authority,
            &permit,
            &runtime_request,
        )
        .unwrap();
    let RuntimeActionOutcome::Resume(outcome) = outcome else {
        panic!("resume routed to the wrong outcome")
    };
    assert!(outcome.root_recovered);
    assert_eq!(
        outcome.current_head.event_count,
        request.expected_head.event_count + 1
    );
    assert_eq!(
        outcome.snapshot.commitments["lease-001"].result_commitment_id,
        commitment
    );
    assert!(!outcome.snapshot.recovery.interrupted_root);
}

#[test]
fn sealed_ambiguity_view_reconciles_one_exact_operation_and_reopens_cleanly() {
    let (root, head) = ambiguous_effect("positive-reconcile");
    let context = context();
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let request = state_request(head, 4);
    let before_read = recursive_fingerprint(root.path());
    let view = adapter.inspect_current(&request).unwrap();
    assert_eq!(recursive_fingerprint(root.path()), before_read);
    let action = view.state().root_action_requests[0].clone();
    assert_eq!(action.operation, RootOperation::Reconcile);
    let authority = authority();
    let reconcile_request = ReconcileRequest {
        expected_head: request.expected_head.clone(),
        tick: request.tick,
        live_workers: request.live_workers.clone(),
        lease_id: "lease-001".to_owned(),
        resolution: not_applied_resolution(),
        target: action.target.clone(),
    };
    let permit = permit_for_reconciliation(
        &authority,
        &action,
        request.tick,
        &reconcile_request.resolution,
    );
    let outcome = adapter
        .execute_reconcile(&view, &action, &authority, &permit, &reconcile_request)
        .unwrap();
    assert_eq!(outcome.settled_operation_id, "operation-001");

    let reopened = adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: outcome.current_head,
            tick: request.tick,
            live_workers: request.live_workers,
        })
        .unwrap();
    assert!(
        reopened
            .state()
            .snapshot
            .recovery
            .ambiguous_operations
            .is_empty()
    );
    assert!(reopened.state().root_action_requests.is_empty());
}

#[test]
fn sealed_interrupted_preview_recovers_exact_append_and_reopens_authoritative_head() {
    let (root, prior, event, request) = interrupted_heartbeat("positive-recover");
    let context = context();
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let before_read = recursive_fingerprint(root.path());
    let view = adapter.inspect_interrupted(&request).unwrap();
    assert_eq!(recursive_fingerprint(root.path()), before_read);
    let action = view.preview().root_action_request.clone();
    assert_eq!(action.operation, RootOperation::Recover);
    let authority = authority();
    let permit = permit_for_action(&authority, &action, request.tick);
    let runtime_request = RuntimeActionRequest::Recover(RecoverRequest {
        expected_prior_head: request.expected_prior_head.clone(),
        expected_event_id: request.expected_event_id.clone(),
        recovered_binding: request.recovered_binding.clone(),
        tick: request.tick,
        live_workers: request.live_workers.clone(),
        target: action.target.clone(),
    });
    let outcome = adapter
        .execute_action(
            RuntimeActionSource::Interrupted(&view),
            &action,
            &authority,
            &permit,
            &runtime_request,
        )
        .unwrap();
    let RuntimeActionOutcome::Recover(outcome) = outcome else {
        panic!("recover routed to the wrong outcome")
    };
    assert_eq!(outcome.recovered_head.last_event_id, event.event_id);
    assert_eq!(outcome.recovered_head.event_count, prior.event_count + 1);
    assert!(outcome.workspace_path_current_after_commit);

    let reopened = adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: outcome.recovered_head,
            tick: request.tick,
            live_workers: BTreeSet::from(["worker-a".to_owned()]),
        })
        .unwrap();
    assert_eq!(reopened.state().snapshot.binding, binding());
    assert_eq!(
        reopened.state().snapshot.event_count,
        prior.event_count as usize + 1
    );
}
