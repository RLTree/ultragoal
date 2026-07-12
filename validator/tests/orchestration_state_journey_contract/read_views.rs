use super::support::*;
use crate::orchestration::product::command::*;
use crate::orchestration::product::*;
use crate::orchestration::*;
use std::collections::BTreeSet;

#[test]
fn inspect_next_and_diagnose_are_deterministic_and_recursively_zero_write() {
    let (root, mut engine) = durable_engine("zero-write", false);
    engine.grant_lease(1, lease(40)).unwrap();
    engine.start(2, "lease-001").unwrap();
    engine.interrupt_root(3).unwrap();
    let head = engine.journal_head().unwrap().clone();
    drop(engine);
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let request = state_request(head, 4);
    let before = recursive_fingerprint(root.path());

    let first = inspect(&context(), &workspace, &request).unwrap();
    let second = inspect(&context(), &workspace, &request).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&second).unwrap()
    );
    assert_eq!(first.workspace_identity, workspace.identity());
    assert_eq!(first.snapshot.binding, binding());
    assert!(first.snapshot.recovery.interrupted_root);

    let selected = next(&first);
    assert_eq!(selected.disposition, NextDisposition::RootAuthorityRequired);
    let action = selected.root_action_request.as_ref().unwrap();
    assert_eq!(action.operation, RootOperation::Resume);
    action.validate_for(&workspace).unwrap();

    let diagnosis = diagnose(&first, selected.finding_id.as_deref())
        .unwrap()
        .unwrap();
    assert_eq!(diagnosis.status, DiagnosisStatus::Finding);
    assert_eq!(diagnosis.root_action_request.as_ref(), Some(action));
    first.validate_action(&workspace, action).unwrap();
    for projection in [
        CommandProjection::Inspect,
        CommandProjection::Next,
        CommandProjection::Diagnose(selected.finding_id.clone()),
    ] {
        let first_bytes = project(&first, &workspace, &projection).unwrap().unwrap();
        let second_bytes = project(&second, &workspace, &projection).unwrap().unwrap();
        assert_eq!(first_bytes, second_bytes);
        let value: serde_json::Value = serde_json::from_slice(&first_bytes).unwrap();
        assert!(value["schema_version"].as_str().unwrap().ends_with("-v1"));
    }
    assert!(
        project(
            &first,
            &workspace,
            &CommandProjection::Diagnose(Some(digest('f')))
        )
        .unwrap()
        .is_none()
    );
    assert_eq!(recursive_fingerprint(root.path()), before);
}

#[test]
fn submitted_result_is_carried_into_the_resume_action_binding() {
    let (root, mut engine) = durable_engine("result-binding", false);
    let artifacts = TestRoot::new("result-binding-artifacts");
    let lease = lease(40);
    let result = worker_result(&artifacts);
    engine.grant_lease(1, lease.clone()).unwrap();
    engine.start(2, &lease.lease_id).unwrap();
    engine
        .submit(
            3,
            &lease.lease_id,
            &result,
            &ArtifactWorkspace::new(artifacts.path()).unwrap(),
        )
        .unwrap();
    let commitment = engine
        .events()
        .iter()
        .find_map(|event| match &event.event {
            EventKind::WorkerSubmitted {
                result_commitment_id,
                ..
            } => Some(result_commitment_id.clone()),
            _ => None,
        })
        .unwrap();
    engine.interrupt_root(4).unwrap();
    let head = engine.journal_head().unwrap().clone();
    drop(engine);

    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let view = inspect(&context(), &workspace, &state_request(head, 5)).unwrap();
    let action = &view.root_action_requests[0];
    assert_eq!(action.target.lease_id.as_deref(), Some("lease-001"));
    assert_eq!(
        action.target.result_commitment_id.as_ref(),
        Some(&commitment)
    );
    action.validate_for(&workspace).unwrap();
}

#[test]
fn clean_state_selects_only_dependency_closed_work_without_root_authority() {
    let (root, engine) = durable_engine("clean-next", false);
    let head = engine.journal_head().unwrap().clone();
    drop(engine);
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let view = inspect(
        &context(),
        &workspace,
        &command_request(head, 1, BTreeSet::new()),
    )
    .unwrap();
    assert!(view.findings.is_empty());
    assert!(view.root_action_requests.is_empty());
    let selected = next(&view);
    assert_eq!(
        selected.disposition,
        NextDisposition::ContinueDependencyClosedWork
    );
    assert_eq!(selected.ready_node.as_deref(), Some("node-a"));
}

#[test]
fn projection_revalidates_the_journal_head_in_the_same_session() {
    let (root, mut engine) = durable_engine("projection-revalidation", false);
    engine.grant_lease(1, lease(40)).unwrap();
    engine.start(2, "lease-001").unwrap();
    let head = engine.journal_head().unwrap().clone();
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let view = inspect(&context(), &workspace, &state_request(head, 2)).unwrap();
    engine.heartbeat(3, "lease-001").unwrap();
    let before = recursive_fingerprint(root.path());
    assert_eq!(
        project(&view, &workspace, &CommandProjection::Inspect).unwrap_err(),
        ProductError::ConcurrentUpdate
    );
    assert_eq!(recursive_fingerprint(root.path()), before);
}

fn command_request(
    expected_head: JournalHead,
    tick: u64,
    live_workers: BTreeSet<String>,
) -> OrchestrationStateRequest {
    OrchestrationStateRequest {
        expected_head,
        tick,
        live_workers,
    }
}
