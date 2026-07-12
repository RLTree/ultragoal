use crate::orchestration::*;
use crate::orchestration_product::*;
use crate::support::*;
use std::collections::BTreeSet;
use std::sync::{Arc, Barrier};

fn pending_effect(label: &str) -> (TestRoot, JournalHead) {
    let (root, mut engine) = durable_engine(label, true);
    engine.grant_lease(1, lease(40)).unwrap();
    engine.start(2, "lease-001").unwrap();
    let request = EffectRequest {
        lease_id: "lease-001".to_owned(),
        binding: binding(),
        effect: effect(EffectClass::Process, "worker-effect"),
        operation_id: "operation-001".to_owned(),
        payload_digest: digest('6'),
    };
    assert_eq!(
        engine.apply_effect(3, request).unwrap_err(),
        OrchestrationError::EffectAmbiguous
    );
    let head = engine.journal_head().unwrap().clone();
    drop(engine);
    (root, head)
}

fn target() -> PermitTarget {
    PermitTarget {
        lease_id: Some("lease-001".to_owned()),
        operation_id: Some("operation-001".to_owned()),
        ..PermitTarget::default()
    }
}

fn request(head: JournalHead) -> ReconcileRequest {
    ReconcileRequest {
        expected_head: head,
        tick: 4,
        live_workers: BTreeSet::from(["worker-a".to_owned()]),
        lease_id: "lease-001".to_owned(),
        resolution: EffectResolution {
            operation_id: "operation-001".to_owned(),
            evidence_digest: digest('e'),
            outcome: EffectOutcome::NotApplied,
        },
        target: target(),
    }
}

fn applied_resolution(receipt_digest: String) -> EffectResolution {
    EffectResolution {
        operation_id: "operation-001".to_owned(),
        evidence_digest: digest('e'),
        outcome: EffectOutcome::Applied {
            receipt: EffectReceipt {
                operation_id: "operation-001".to_owned(),
                effect: effect(EffectClass::Process, "worker-effect"),
                receipt_digest,
            },
        },
    }
}

#[test]
fn concurrent_reconciliation_has_one_authoritative_outcome_and_no_orphan() {
    let (root, head) = pending_effect("concurrent-reconcile");
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let authority = authority();
    let not_applied = request(head.clone()).resolution;
    let applied = applied_resolution(digest('d'));
    let permits_and_requests = [
        (
            reconcile_permit(&authority, &workspace, &head, 4, target(), &not_applied),
            ReconcileRequest {
                resolution: not_applied,
                ..request(head.clone())
            },
        ),
        (
            reconcile_permit(&authority, &workspace, &head, 4, target(), &applied),
            ReconcileRequest {
                resolution: applied,
                ..request(head.clone())
            },
        ),
    ];
    let barrier = Arc::new(Barrier::new(2));
    let handles = permits_and_requests
        .into_iter()
        .map(|(permit, request)| {
            let barrier = Arc::clone(&barrier);
            let context = context();
            let workspace = workspace.clone();
            let authority = authority.clone();
            std::thread::spawn(move || {
                barrier.wait();
                reconcile(&context, &workspace, &authority, &permit, &request)
            })
        })
        .collect::<Vec<_>>();
    let results = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
    assert!(
        results
            .iter()
            .filter_map(|result| result.as_ref().err())
            .all(|error| matches!(
                error,
                ProductError::ConcurrentUpdate | ProductError::UnknownOperation
            ))
    );
    let accepted = results.into_iter().find_map(Result::ok).unwrap();
    let current = query(
        &context(),
        &workspace,
        &QueryRequest {
            expected_head: accepted.current_head,
            tick: 4,
            live_workers: BTreeSet::from(["worker-a".to_owned()]),
        },
    )
    .unwrap();
    assert!(current.recovery.ambiguous_operations.is_empty());
    assert!(current.settled_operations.contains("operation-001"));
}

#[test]
fn reconcile_permit_binds_evidence_outcome_and_complete_receipt_before_mutation() {
    let (root, head) = pending_effect("decision-binding");
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let authority = authority();
    let exact_resolution = applied_resolution(digest('d'));
    let permit = reconcile_permit(
        &authority,
        &workspace,
        &head,
        4,
        target(),
        &exact_resolution,
    );
    let exact = ReconcileRequest {
        resolution: exact_resolution.clone(),
        ..request(head.clone())
    };
    let before = recursive_fingerprint(root.path());

    let mut evidence_substitution = exact.clone();
    evidence_substitution.resolution.evidence_digest = digest('f');
    let mut outcome_substitution = exact.clone();
    outcome_substitution.resolution.outcome = EffectOutcome::NotApplied;
    let mut receipt_digest_substitution = exact.clone();
    let EffectOutcome::Applied { receipt } = &mut receipt_digest_substitution.resolution.outcome
    else {
        unreachable!()
    };
    receipt.receipt_digest = digest('f');
    let mut receipt_effect_substitution = exact.clone();
    let EffectOutcome::Applied { receipt } = &mut receipt_effect_substitution.resolution.outcome
    else {
        unreachable!()
    };
    receipt.effect = effect(EffectClass::Process, "different-effect");

    for substitution in [
        evidence_substitution,
        outcome_substitution,
        receipt_digest_substitution,
        receipt_effect_substitution,
    ] {
        assert_eq!(
            reconcile(&context(), &workspace, &authority, &permit, &substitution,).unwrap_err(),
            ProductError::AuthorityInvalid
        );
        assert_eq!(recursive_fingerprint(root.path()), before);
    }

    let outcome = reconcile(&context(), &workspace, &authority, &permit, &exact).unwrap();
    assert_eq!(outcome.settled_operation_id, "operation-001");
}

#[test]
fn legacy_or_action_only_permits_cannot_authorize_reconciliation() {
    let (root, head) = pending_effect("legacy-permit");
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let authority = authority();
    let exact = request(head.clone());
    let permit = reconcile_permit(
        &authority,
        &workspace,
        &head,
        4,
        target(),
        &exact.resolution,
    );
    let before = recursive_fingerprint(root.path());

    let mut legacy_value = serde_json::to_value(&permit).unwrap();
    legacy_value["schema_version"] =
        serde_json::Value::String("OrchestrationRootPermit-v1".to_owned());
    let legacy: RootPermit = serde_json::from_value(legacy_value.clone()).unwrap();
    assert_eq!(
        reconcile(&context(), &workspace, &authority, &legacy, &exact).unwrap_err(),
        ProductError::AuthorityInvalid
    );
    legacy_value
        .as_object_mut()
        .unwrap()
        .remove("decision_binding");
    assert!(serde_json::from_value::<RootPermit>(legacy_value).is_err());
    assert_eq!(recursive_fingerprint(root.path()), before);

    assert_eq!(
        issue_action_permit_for_test(
            &authority,
            RootOperation::Reconcile,
            binding(),
            workspace.identity(),
            &journal_head_identity(&head).unwrap(),
            3,
            14,
            b"action-only-reconcile-nonce-0123",
            target(),
        ),
        Err(ProductError::AuthorityOperationMismatch)
    );
    assert_eq!(recursive_fingerprint(root.path()), before);
}

#[test]
fn ambiguous_resume_requires_reconciliation_and_writes_nothing() {
    let (root, head) = pending_effect("ambiguous-resume");
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let authority = authority();
    let resume_target = PermitTarget {
        lease_id: Some("lease-001".to_owned()),
        ..PermitTarget::default()
    };
    let permit = permit(
        &authority,
        RootOperation::Resume,
        &workspace,
        &head,
        4,
        resume_target.clone(),
    );
    let before = recursive_fingerprint(root.path());
    let error = resume(
        &context(),
        &workspace,
        &authority,
        &permit,
        &ResumeRequest {
            expected_head: head,
            tick: 4,
            live_workers: BTreeSet::from(["worker-a".to_owned()]),
            target: resume_target,
        },
    )
    .unwrap_err();
    assert_eq!(error, ProductError::AmbiguousRecovery);
    assert_eq!(recursive_fingerprint(root.path()), before);
}

#[test]
fn reconciliation_receipt_without_pending_behavior_is_rejected() {
    let (root, mut engine) = durable_engine("receipt-without-behavior", false);
    engine.grant_lease(1, lease(40)).unwrap();
    engine.start(2, "lease-001").unwrap();
    let head = engine.journal_head().unwrap().clone();
    drop(engine);
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let authority = authority();
    let decision = request(head.clone()).resolution;
    let permit = reconcile_permit(&authority, &workspace, &head, 4, target(), &decision);
    let before = recursive_fingerprint(root.path());
    let error = reconcile(&context(), &workspace, &authority, &permit, &request(head)).unwrap_err();
    assert_eq!(error, ProductError::UnknownOperation);
    assert_eq!(recursive_fingerprint(root.path()), before);
}
