use super::support::*;
use crate::orchestration::product::command::*;
use crate::orchestration::product::*;
use crate::orchestration::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

fn running(label: &str, deadline: u64) -> (TestRoot, JournalHead) {
    let (root, mut engine) = durable_engine(label, false);
    engine.grant_lease(1, lease(deadline)).unwrap();
    engine.start(2, "lease-001").unwrap();
    let head = engine.journal_head().unwrap().clone();
    (root, head)
}

#[test]
fn unknown_worker_and_stale_candidate_refuse_without_mutation() {
    let (root, head) = running("unknown-stale", 40);
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let before = recursive_fingerprint(root.path());
    let unknown = OrchestrationStateRequest {
        expected_head: head.clone(),
        tick: 3,
        live_workers: BTreeSet::from(["unknown-worker".to_owned()]),
    };
    assert_eq!(
        inspect(&context(), &workspace, &unknown).unwrap_err(),
        ProductError::UnknownWorker
    );
    let stale = ProductContext::new(
        graph(),
        policy(),
        Binding::new(&digest('c'), &digest('d')).unwrap(),
        root_actor(),
    );
    assert_eq!(
        inspect(&stale, &workspace, &state_request(head, 3)).unwrap_err(),
        ProductError::StaleCandidate
    );
    assert_eq!(recursive_fingerprint(root.path()), before);
}

#[test]
fn expired_and_orphaned_leases_produce_causal_no_route_views() {
    for (label, tick, live, expected) in [
        (
            "expired",
            4,
            BTreeSet::from(["worker-a".to_owned()]),
            FindingKind::ExpiredLease,
        ),
        ("orphaned", 3, BTreeSet::new(), FindingKind::OrphanedLease),
    ] {
        let (root, head) = running(label, 3);
        let workspace = ProductWorkspace::open(root.path()).unwrap();
        let request = OrchestrationStateRequest {
            expected_head: head,
            tick,
            live_workers: live,
        };
        let before = recursive_fingerprint(root.path());
        let view = inspect(&context(), &workspace, &request).unwrap();
        assert_eq!(view.findings[0].kind, expected);
        assert!(view.root_action_requests.is_empty());
        assert_eq!(next(&view).disposition, NextDisposition::NoLegalRoute);
        assert_eq!(recursive_fingerprint(root.path()), before);
    }
}

#[test]
fn serialized_action_substitution_is_rejected_before_mutation() {
    let (root, mut engine) = durable_engine("action-substitution", false);
    engine.interrupt_root(1).unwrap();
    let head = engine.journal_head().unwrap().clone();
    drop(engine);
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let view = inspect(
        &context(),
        &workspace,
        &OrchestrationStateRequest {
            expected_head: head,
            tick: 2,
            live_workers: BTreeSet::new(),
        },
    )
    .unwrap();
    let before = recursive_fingerprint(root.path());
    let mut value = serde_json::to_value(&view.root_action_requests[0]).unwrap();
    value["target"]["lease_id"] = serde_json::Value::String("forged-lease".to_owned());
    let substituted: RootActionRequest = serde_json::from_value(value).unwrap();
    assert_eq!(
        substituted.validate_for(&workspace).unwrap_err(),
        ProductError::AuthorityInvalid
    );

    let mut self_consistent = view.root_action_requests[0].clone();
    self_consistent.snapshot_id = digest('f');
    self_consistent.action_id = recompute_action_id(&self_consistent);
    self_consistent.validate_for(&workspace).unwrap();
    assert_eq!(
        view.validate_action(&workspace, &self_consistent)
            .unwrap_err(),
        ProductError::AuthorityInvalid
    );
    assert_eq!(recursive_fingerprint(root.path()), before);
}

#[test]
fn whole_view_and_cloned_full_state_substitutions_are_rejected_without_mutation() {
    let (root, engine) = durable_engine("whole-view-substitution", false);
    let head = engine.journal_head().unwrap().clone();
    drop(engine);
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let view = inspect(
        &context(),
        &workspace,
        &OrchestrationStateRequest {
            expected_head: head,
            tick: 1,
            live_workers: BTreeSet::new(),
        },
    )
    .unwrap();
    let before = recursive_fingerprint(root.path());
    assert_eq!(view.state_id, recompute_state_id(&view));

    let round_tripped: OrchestrationStateView =
        serde_json::from_slice(&serde_json::to_vec(&view).unwrap()).unwrap();
    assert_eq!(
        project(&round_tripped, &workspace, &CommandProjection::Inspect).unwrap_err(),
        ProductError::AuthorityInvalid
    );

    let mut cloned = view.clone();
    cloned.snapshot.plan.ready = vec!["forged-ready-node".to_owned()];
    cloned.snapshot_id = cloned.snapshot.snapshot_id().unwrap();
    cloned.state_id = recompute_state_id(&cloned);
    assert_eq!(
        project(&cloned, &workspace, &CommandProjection::Inspect).unwrap_err(),
        ProductError::AuthorityInvalid
    );
    let refused = next(&cloned);
    assert_eq!(refused.disposition, NextDisposition::NoLegalRoute);
    assert!(refused.ready_node.is_none());

    let mut mixed = view.clone();
    mixed.snapshot.event_count = 0;
    mixed.snapshot_id = mixed.snapshot.snapshot_id().unwrap();
    mixed.state_id = recompute_state_id(&mixed);
    assert_eq!(
        project(&mixed, &workspace, &CommandProjection::Inspect).unwrap_err(),
        ProductError::AuthorityInvalid
    );

    let mut serialized = serde_json::to_value(&view).unwrap();
    serialized["snapshot"]["plan"]["ready"] = serde_json::json!(["forged-ready-node"]);
    let forged_snapshot: ProductSnapshot =
        serde_json::from_value(serialized["snapshot"].clone()).unwrap();
    serialized["snapshot_id"] = serde_json::Value::String(forged_snapshot.snapshot_id().unwrap());
    let mut deserialized: OrchestrationStateView = serde_json::from_value(serialized).unwrap();
    deserialized.state_id = recompute_state_id(&deserialized);
    assert_eq!(
        project(&deserialized, &workspace, &CommandProjection::Next).unwrap_err(),
        ProductError::AuthorityInvalid
    );
    assert_eq!(recursive_fingerprint(root.path()), before);
}

#[test]
fn finding_and_root_action_membership_injection_cannot_forge_full_state_authority() {
    let (root, mut engine) = durable_engine("view-membership-substitution", false);
    engine.interrupt_root(1).unwrap();
    let head = engine.journal_head().unwrap().clone();
    drop(engine);
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let view = inspect(
        &context(),
        &workspace,
        &OrchestrationStateRequest {
            expected_head: head,
            tick: 2,
            live_workers: BTreeSet::new(),
        },
    )
    .unwrap();
    let before = recursive_fingerprint(root.path());

    let mut deleted = view.clone();
    deleted.findings.clear();
    deleted.root_action_requests.clear();
    deleted.state_id = recompute_state_id(&deleted);
    assert_eq!(
        diagnose(&deleted, None).unwrap_err(),
        ProductError::AuthorityInvalid
    );

    let mut finding_injected = view.clone();
    let mut finding = finding_injected.findings[0].clone();
    finding.finding_id = digest('e');
    finding.code = "HUL-ORCH-STATE-FORGED".to_owned();
    finding_injected.findings.push(finding);
    finding_injected.state_id = recompute_state_id(&finding_injected);
    assert_eq!(
        diagnose(&finding_injected, None).unwrap_err(),
        ProductError::AuthorityInvalid
    );

    let mut injected = view.clone();
    let mut action = injected.root_action_requests[0].clone();
    action.snapshot_id = digest('f');
    action.action_id = recompute_action_id(&action);
    injected.root_action_requests.push(action.clone());
    injected.state_id = recompute_state_id(&injected);
    assert_eq!(
        injected.validate_action(&workspace, &action).unwrap_err(),
        ProductError::AuthorityInvalid
    );
    assert_eq!(recursive_fingerprint(root.path()), before);
}

#[test]
fn interrupted_json_snapshot_and_snapshot_id_substitution_are_untrusted() {
    let (root, workspace, _request, view) = interrupted_with_result("interrupted-view-seal");

    let round_tripped: InterruptedRecoveryView =
        serde_json::from_slice(&serde_json::to_vec(&view).unwrap()).unwrap();
    let round_tripped_action = round_tripped.root_action_request.clone();
    assert_interrupted_rejected(
        &root,
        &workspace,
        &round_tripped,
        &round_tripped_action,
        ProductError::AuthorityInvalid,
    );

    let mut ready_substitution = view.clone();
    ready_substitution.snapshot.plan.ready = vec!["forged-ready-node".to_owned()];
    ready_substitution.snapshot_id = ready_substitution.snapshot.snapshot_id().unwrap();
    ready_substitution.root_action_request.snapshot_id = ready_substitution.snapshot_id.clone();
    ready_substitution.root_action_request.action_id =
        recompute_action_id(&ready_substitution.root_action_request);
    let ready_action = ready_substitution.root_action_request.clone();
    assert_interrupted_rejected(
        &root,
        &workspace,
        &ready_substitution,
        &ready_action,
        ProductError::AuthorityInvalid,
    );

    let mut snapshot_id_substitution = view.clone();
    snapshot_id_substitution.snapshot_id = digest('f');
    snapshot_id_substitution.root_action_request.snapshot_id = digest('f');
    snapshot_id_substitution.root_action_request.action_id =
        recompute_action_id(&snapshot_id_substitution.root_action_request);
    let snapshot_id_action = snapshot_id_substitution.root_action_request.clone();
    assert_interrupted_rejected(
        &root,
        &workspace,
        &snapshot_id_substitution,
        &snapshot_id_action,
        ProductError::AuthorityInvalid,
    );

    let mut non_echoing = view.clone();
    non_echoing.workspace_identity = "attacker-private-canary".to_owned();
    let action = non_echoing.root_action_request.clone();
    let before = recursive_fingerprint(root.path());
    let issued = AtomicUsize::new(0);
    let error = validate_then_issue(&non_echoing, &workspace, &action, &issued).unwrap_err();
    assert_eq!(error, ProductError::AuthorityInvalid);
    assert!(!error.to_string().contains("attacker-private-canary"));
    assert_eq!(issued.load(Ordering::SeqCst), 0);
    assert_eq!(recursive_fingerprint(root.path()), before);
}

#[test]
fn interrupted_lease_and_result_substitution_cannot_rewrite_the_recovery_target() {
    let (root, workspace, _request, view) = interrupted_with_result("interrupted-target-seal");
    assert_eq!(
        view.root_action_request.target.lease_id.as_deref(),
        Some("lease-001")
    );
    assert!(
        view.root_action_request
            .target
            .result_commitment_id
            .is_some()
    );

    let mut stripped = view.clone();
    stripped.root_action_request.target.lease_id = None;
    stripped.root_action_request.target.result_commitment_id = None;
    stripped.root_action_request.action_id = recompute_action_id(&stripped.root_action_request);
    let stripped_action = stripped.root_action_request.clone();
    assert_interrupted_rejected(
        &root,
        &workspace,
        &stripped,
        &stripped_action,
        ProductError::AuthorityInvalid,
    );

    let mut alternate = view.clone();
    let mut alternate_commitment = alternate.snapshot.commitments["lease-001"].clone();
    alternate_commitment.result_commitment_id = digest('e');
    alternate
        .snapshot
        .commitments
        .insert("lease-002".to_owned(), alternate_commitment);
    alternate.snapshot_id = alternate.snapshot.snapshot_id().unwrap();
    alternate.root_action_request.snapshot_id = alternate.snapshot_id.clone();
    alternate.root_action_request.target.lease_id = Some("lease-002".to_owned());
    alternate.root_action_request.target.result_commitment_id = Some(digest('e'));
    alternate.root_action_request.action_id = recompute_action_id(&alternate.root_action_request);
    let alternate_action = alternate.root_action_request.clone();
    assert_interrupted_rejected(
        &root,
        &workspace,
        &alternate,
        &alternate_action,
        ProductError::AuthorityInvalid,
    );
}

#[test]
fn interrupted_head_identity_and_current_source_are_bound_before_issuance() {
    let (root, workspace, request, view) = interrupted_with_result("interrupted-head-seal");
    let exact_action = view.root_action_request.clone();
    let before = recursive_fingerprint(root.path());
    let issued = AtomicUsize::new(0);
    let permit = validate_then_issue(&view, &workspace, &exact_action, &issued).unwrap();
    assert_eq!(issued.load(Ordering::SeqCst), 1);
    assert_eq!(
        serde_json::to_value(&permit).unwrap()["target"],
        serde_json::to_value(&exact_action.target).unwrap()
    );
    assert_eq!(recursive_fingerprint(root.path()), before);

    let mut prospective_substitution = view.clone();
    prospective_substitution.prospective_head.log_sha256 = digest('e');
    prospective_substitution.snapshot.journal_head =
        prospective_substitution.prospective_head.clone();
    prospective_substitution.snapshot_id = prospective_substitution.snapshot.snapshot_id().unwrap();
    prospective_substitution.root_action_request.snapshot_id =
        prospective_substitution.snapshot_id.clone();
    prospective_substitution.root_action_request.action_id =
        recompute_action_id(&prospective_substitution.root_action_request);
    let prospective_action = prospective_substitution.root_action_request.clone();
    assert_interrupted_rejected(
        &root,
        &workspace,
        &prospective_substitution,
        &prospective_action,
        ProductError::AuthorityInvalid,
    );

    let mut identity_substitution = view.clone();
    identity_substitution.prospective_journal_head_identity = digest('d');
    let identity_action = identity_substitution.root_action_request.clone();
    assert_interrupted_rejected(
        &root,
        &workspace,
        &identity_substitution,
        &identity_action,
        ProductError::AuthorityInvalid,
    );

    let mut prior_substitution = view.clone();
    prior_substitution.prior_head.last_event_id = digest('c');
    prior_substitution.prior_journal_head_identity =
        journal_head_identity(&prior_substitution.prior_head).unwrap();
    prior_substitution.root_action_request.expected_head = prior_substitution.prior_head.clone();
    prior_substitution.root_action_request.journal_head_identity =
        prior_substitution.prior_journal_head_identity.clone();
    prior_substitution.root_action_request.action_id =
        recompute_action_id(&prior_substitution.root_action_request);
    let prior_action = prior_substitution.root_action_request.clone();
    assert_interrupted_rejected(
        &root,
        &workspace,
        &prior_substitution,
        &prior_action,
        ProductError::AuthorityInvalid,
    );

    FileJournal::recover_interrupted_append(
        root.path(),
        &request.expected_prior_head,
        &request.expected_event_id,
        &request.recovered_binding,
    )
    .unwrap();
    assert_interrupted_rejected(
        &root,
        &workspace,
        &view,
        &exact_action,
        ProductError::ConcurrentUpdate,
    );
}

fn interrupted_with_result(
    label: &str,
) -> (
    TestRoot,
    ProductWorkspace,
    InterruptedRecoveryRequest,
    InterruptedRecoveryView,
) {
    let (root, mut engine) = durable_engine(label, false);
    let artifacts = TestRoot::new(&format!("{label}-artifacts"));
    let lease = lease(40);
    let result = worker_result(&artifacts);
    engine.grant_lease(1, lease.clone()).unwrap();
    engine.start(2, &lease.lease_id).unwrap();
    let prior = engine.journal_head().unwrap().clone();
    let commitment =
        result_commitment_from_parts(&binding(), "node-a", &lease.lease_id, &result).unwrap();
    let result_commitment_id = commitment.commitment_id().unwrap();
    let event = engine
        .event_log()
        .next(
            &binding(),
            worker_actor(),
            3,
            EventKind::WorkerSubmitted {
                lease_id: lease.lease_id,
                commitment,
                result_commitment_id,
            },
        )
        .unwrap();
    drop(engine);

    let mut bytes = fs::read(root.path().join("events.jsonl")).unwrap();
    bytes.extend(journal_frame_bytes(&event));
    bytes.push(b'\n');
    fs::write(root.path().join("events.jsonl"), bytes).unwrap();

    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let request = InterruptedRecoveryRequest {
        expected_prior_head: prior,
        expected_event_id: event.event_id,
        recovered_binding: binding(),
        tick: 3,
        live_workers: BTreeSet::from(["worker-a".to_owned()]),
    };
    let view = inspect_interrupted(&context(), &workspace, &request).unwrap();
    (root, workspace, request, view)
}

fn journal_frame_bytes(event: &OrchestrationEvent) -> Vec<u8> {
    #[derive(Serialize)]
    struct Frame<'a> {
        schema_version: &'static str,
        event: &'a OrchestrationEvent,
    }
    serde_json::to_vec(&Frame {
        schema_version: "OrchestrationJournalFrame-v1",
        event,
    })
    .unwrap()
}

fn validate_then_issue(
    view: &InterruptedRecoveryView,
    workspace: &ProductWorkspace,
    action: &RootActionRequest,
    issued: &AtomicUsize,
) -> Result<RootPermit, ProductError> {
    view.validate_action(workspace, action)?;
    issued.fetch_add(1, Ordering::SeqCst);
    Ok(permit_for_action(action, 4).1)
}

fn assert_interrupted_rejected(
    root: &TestRoot,
    workspace: &ProductWorkspace,
    view: &InterruptedRecoveryView,
    action: &RootActionRequest,
    expected: ProductError,
) {
    let before = recursive_fingerprint(root.path());
    let issued = AtomicUsize::new(0);
    assert_eq!(
        validate_then_issue(view, workspace, action, &issued).unwrap_err(),
        expected
    );
    assert_eq!(issued.load(Ordering::SeqCst), 0);
    assert_eq!(recursive_fingerprint(root.path()), before);
}

fn recompute_state_id(view: &OrchestrationStateView) -> String {
    let bytes = serde_json::to_vec(&(
        &view.schema_version,
        &view.workspace_identity,
        &view.journal_head_identity,
        &view.snapshot_id,
        &view.snapshot,
        &view.findings,
        &view.root_action_requests,
    ))
    .unwrap();
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn recompute_action_id(action: &RootActionRequest) -> String {
    #[derive(Serialize)]
    struct Commitment<'a> {
        schema_version: &'a str,
        operation: RootOperation,
        reason: RootActionReason,
        authority_binding: &'a Binding,
        workspace_identity: &'a str,
        journal_head_identity: &'a str,
        expected_head: &'a JournalHead,
        snapshot_id: &'a str,
        target: &'a PermitTarget,
    }
    let bytes = serde_json::to_vec(&Commitment {
        schema_version: &action.schema_version,
        operation: action.operation,
        reason: action.reason,
        authority_binding: &action.authority_binding,
        workspace_identity: &action.workspace_identity,
        journal_head_identity: &action.journal_head_identity,
        expected_head: &action.expected_head,
        snapshot_id: &action.snapshot_id,
        target: &action.target,
    })
    .unwrap();
    format!("sha256:{:x}", Sha256::digest(bytes))
}
