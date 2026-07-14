use super::fixture::*;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use ultragoal::orchestration::product::command::OrchestrationStateRequest;
use ultragoal::orchestration::product::runtime_adapter::{
    OrchestrationRuntimeAdapter, RuntimeActionOutcome, RuntimeActionRequest, RuntimeActionSource,
};
use ultragoal::orchestration::product::{
    PermitReplayState, ProductError, ProductWorkspace, ProductionRootAuthority, ResumeRequest,
    RootPermit,
};

#[test]
fn cross_root_same_head_and_signed_permit_substitutions_never_settle_origin_ledger() {
    if std::env::var_os(CHILD_ENV).is_some() {
        return;
    }
    let (origin, head) = interrupted_root("settlement-origin");
    let clone = clone_journal(&origin, "settlement-clone");
    let authority_root = TestRoot::new("settlement-authority", 0o700);
    let (_, permit, authority) = issue_resume(&authority_root, &origin, head.clone(), 2);
    assert_eq!(
        authority.replay_state(&permit).unwrap(),
        Some(PermitReplayState::Issued)
    );
    let context = context();
    let workspace = ProductWorkspace::open(clone.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let view = adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: head,
            tick: 3,
            live_workers: BTreeSet::new(),
        })
        .unwrap();
    let before = recursive_fingerprint(authority_root.path());
    assert_eq!(
        adapter
            .reconcile_production_reservation(&authority, &view, &permit)
            .unwrap_err(),
        ProductError::AuthorityInvalid
    );
    assert_eq!(recursive_fingerprint(authority_root.path()), before);

    let origin_workspace = ProductWorkspace::open(origin.path()).unwrap();
    let origin_adapter = OrchestrationRuntimeAdapter::new(&context, &origin_workspace).unwrap();
    let origin_view = origin_adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: view.state().snapshot.journal_head.clone(),
            tick: 3,
            live_workers: BTreeSet::new(),
        })
        .unwrap();
    for substituted in substituted_permits(&permit) {
        assert_eq!(
            origin_adapter
                .reconcile_production_reservation(&authority, &origin_view, &substituted)
                .unwrap_err(),
            ProductError::AuthorityInvalid
        );
        assert_eq!(recursive_fingerprint(authority_root.path()), before);
    }
    assert_eq!(
        authority.replay_state(&permit).unwrap(),
        Some(PermitReplayState::Issued)
    );
}

#[test]
fn stale_clone_after_origin_effect_cannot_mutate_committed_permit() {
    if std::env::var_os(CHILD_ENV).is_some() {
        return;
    }
    let (origin, head) = interrupted_root("stale-clone-origin");
    let clone = clone_journal(&origin, "stale-clone-copy");
    let authority_root = TestRoot::new("stale-clone-authority", 0o700);
    let (action, permit, authority) = issue_resume(&authority_root, &origin, head.clone(), 2);
    let context = context();
    let origin_workspace = ProductWorkspace::open(origin.path()).unwrap();
    let origin_adapter = OrchestrationRuntimeAdapter::new(&context, &origin_workspace).unwrap();
    let origin_view = origin_adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: head.clone(),
            tick: 3,
            live_workers: BTreeSet::new(),
        })
        .unwrap();
    let outcome = origin_adapter
        .execute_production_action(
            &authority,
            RuntimeActionSource::Current(&origin_view),
            &action,
            &permit,
            &RuntimeActionRequest::Resume(ResumeRequest {
                expected_head: head.clone(),
                tick: 3,
                live_workers: BTreeSet::new(),
                target: action.target.clone(),
            }),
        )
        .unwrap();
    assert!(matches!(outcome, RuntimeActionOutcome::Resume(_)));

    let clone_workspace = ProductWorkspace::open(clone.path()).unwrap();
    let clone_adapter = OrchestrationRuntimeAdapter::new(&context, &clone_workspace).unwrap();
    let stale_view = clone_adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: head,
            tick: 4,
            live_workers: BTreeSet::new(),
        })
        .unwrap();
    let before = recursive_fingerprint(authority_root.path());
    assert_eq!(
        clone_adapter
            .reconcile_production_reservation(&authority, &stale_view, &permit)
            .unwrap_err(),
        ProductError::AuthorityInvalid
    );
    assert_eq!(recursive_fingerprint(authority_root.path()), before);
    assert_eq!(
        authority.replay_state(&permit).unwrap(),
        Some(PermitReplayState::Committed)
    );
}

fn clone_journal(origin: &TestRoot, label: &str) -> TestRoot {
    let clone = TestRoot::new(label, 0o700);
    for entry in fs::read_dir(origin.path()).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), clone.path().join(entry.file_name())).unwrap();
    }
    clone
}

fn substituted_permits(permit: &RootPermit) -> Vec<RootPermit> {
    let original = serde_json::to_value(permit).unwrap();
    let mut substitutions = [
        ("root_actor", Value::String("other-root".to_owned())),
        (
            "workspace_identity",
            Value::String(format!("sha256:{}", "b".repeat(64))),
        ),
        (
            "journal_head_identity",
            Value::String(format!("sha256:{}", "c".repeat(64))),
        ),
    ]
    .into_iter()
    .map(|(field, value)| {
        let mut changed = original.clone();
        changed[field] = value;
        serde_json::from_value(changed).unwrap()
    })
    .collect::<Vec<_>>();
    let mut wrong_binding = original.clone();
    wrong_binding["binding"]["candidate_id"] = Value::String(format!("sha256:{}", "d".repeat(64)));
    substitutions.push(serde_json::from_value(wrong_binding).unwrap());
    let mut wrong_target = original;
    wrong_target["target"]["lease_id"] = Value::String("other-lease".to_owned());
    substitutions.push(serde_json::from_value(wrong_target).unwrap());
    substitutions
}
