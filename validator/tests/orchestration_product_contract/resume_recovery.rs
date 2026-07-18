use super::product_fixture::*;
use crate::orchestration::product::*;
use crate::orchestration::*;
use std::collections::BTreeSet;

fn submitted_interrupted(label: &str) -> (TestRoot, JournalHead, String) {
    let (root, mut engine) = durable_engine(label, false);
    let artifacts = TestRoot::new(&format!("{label}-artifacts"));
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
    (root, head, commitment)
}

#[test]
fn authorized_resume_preserves_candidate_lease_and_result_binding() {
    let (root, head, commitment) = submitted_interrupted("authorized-resume");
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let authority = authority();
    let target = PermitTarget {
        lease_id: Some("lease-001".to_owned()),
        result_commitment_id: Some(commitment.clone()),
        ..PermitTarget::default()
    };
    let resume_permit = permit(
        &authority,
        RootOperation::Resume,
        &workspace,
        &head,
        5,
        target.clone(),
    );
    let outcome = resume(
        &context(),
        &workspace,
        &authority,
        &resume_permit,
        &ResumeRequest {
            expected_head: head.clone(),
            tick: 5,
            live_workers: BTreeSet::from(["worker-a".to_owned()]),
            target,
        },
    )
    .unwrap();
    assert!(outcome.root_recovered);
    assert_eq!(outcome.current_head.event_count, head.event_count + 1);
    assert_eq!(outcome.snapshot.binding, binding());
    assert_eq!(
        outcome.snapshot.commitments["lease-001"].result_commitment_id,
        commitment
    );
    assert!(!outcome.snapshot.recovery.interrupted_root);
}

#[test]
fn stale_result_expired_authority_and_expired_lease_fail_closed() {
    let (root, head, commitment) = submitted_interrupted("resume-negative");
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let authority = authority();
    let before = recursive_fingerprint(root.path());
    let wrong_target = PermitTarget {
        lease_id: Some("lease-001".to_owned()),
        result_commitment_id: Some(digest('f')),
        ..PermitTarget::default()
    };
    let wrong_permit = permit(
        &authority,
        RootOperation::Resume,
        &workspace,
        &head,
        5,
        wrong_target.clone(),
    );
    let error = resume(
        &context(),
        &workspace,
        &authority,
        &wrong_permit,
        &ResumeRequest {
            expected_head: head.clone(),
            tick: 5,
            live_workers: BTreeSet::from(["worker-a".to_owned()]),
            target: wrong_target,
        },
    )
    .unwrap_err();
    assert_eq!(error, ProductError::ResultSubstitution);
    assert_eq!(recursive_fingerprint(root.path()), before);

    let target = PermitTarget {
        lease_id: Some("lease-001".to_owned()),
        result_commitment_id: Some(commitment),
        ..PermitTarget::default()
    };
    let expired = issue_action_permit_for_test(
        &authority,
        RootActionPermitIssuance {
            operation: RootOperation::Resume,
            binding: binding(),
            workspace_identity: workspace.identity(),
            journal_head_identity: &journal_head_identity(&head).unwrap(),
            issued_tick: 1,
            expires_tick: 2,
            nonce: b"another-unique-nonce-0123456",
            target: target.clone(),
        },
    )
    .unwrap();
    assert_eq!(
        resume(
            &context(),
            &workspace,
            &authority,
            &expired,
            &ResumeRequest {
                expected_head: head,
                tick: 5,
                live_workers: BTreeSet::from(["worker-a".to_owned()]),
                target,
            },
        )
        .unwrap_err(),
        ProductError::AuthorityExpired
    );

    let (expired_root, mut engine) = durable_engine("expired-lease", false);
    engine.grant_lease(1, lease(3)).unwrap();
    engine.start(2, "lease-001").unwrap();
    let expired_head = engine.journal_head().unwrap().clone();
    drop(engine);
    let expired_workspace = ProductWorkspace::open(expired_root.path()).unwrap();
    let target = PermitTarget {
        lease_id: Some("lease-001".to_owned()),
        ..PermitTarget::default()
    };
    let expired_lease_permit = permit(
        &authority,
        RootOperation::Resume,
        &expired_workspace,
        &expired_head,
        4,
        target.clone(),
    );
    assert_eq!(
        resume(
            &context(),
            &expired_workspace,
            &authority,
            &expired_lease_permit,
            &ResumeRequest {
                expected_head: expired_head,
                tick: 4,
                live_workers: BTreeSet::from(["worker-a".to_owned()]),
                target,
            },
        )
        .unwrap_err(),
        ProductError::LeaseExpired
    );
}
