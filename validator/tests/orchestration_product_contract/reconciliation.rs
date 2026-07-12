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

#[test]
fn concurrent_reconciliation_has_one_authoritative_outcome_and_no_orphan() {
    let (root, head) = pending_effect("concurrent-reconcile");
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let authority = authority();
    let permit = permit(
        &authority,
        RootOperation::Reconcile,
        &workspace,
        &head,
        4,
        target(),
    );
    let barrier = Arc::new(Barrier::new(2));
    let handles = (0..2)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            let context = context();
            let workspace = workspace.clone();
            let authority = authority.clone();
            let permit = permit.clone();
            let request = request(head.clone());
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
    let permit = permit(
        &authority,
        RootOperation::Reconcile,
        &workspace,
        &head,
        4,
        target(),
    );
    let before = recursive_fingerprint(root.path());
    let error = reconcile(&context(), &workspace, &authority, &permit, &request(head)).unwrap_err();
    assert_eq!(error, ProductError::UnknownOperation);
    assert_eq!(recursive_fingerprint(root.path()), before);
}
