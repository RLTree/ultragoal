use super::product_fixture::*;
use crate::orchestration::product::*;
use crate::orchestration::*;
use std::collections::BTreeSet;

fn running(label: &str) -> (TestRoot, JournalHead) {
    let (root, mut engine) = durable_engine(label, false);
    engine.grant_lease(1, lease(40)).unwrap();
    engine.start(2, "lease-001").unwrap();
    let head = engine.journal_head().unwrap().clone();
    (root, head)
}

fn submitted(label: &str) -> (TestRoot, JournalHead, String) {
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
    let head = engine.journal_head().unwrap().clone();
    (root, head, commitment)
}

#[test]
fn authority_token_mutation_and_secret_echo_fail_closed() {
    let (root, head) = running("authority-mutation");
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let authority = authority();
    let target = PermitTarget {
        lease_id: Some("lease-001".to_owned()),
        ..PermitTarget::default()
    };
    let permit = permit(
        &authority,
        RootOperation::Resume,
        &workspace,
        &head,
        3,
        target.clone(),
    );
    let mut value = serde_json::to_value(&permit).unwrap();
    value["authenticator"] = serde_json::Value::String(digest('f'));
    let tampered: RootPermit = serde_json::from_value(value).unwrap();
    let before = recursive_fingerprint(root.path());
    let error = resume(
        &context(),
        &workspace,
        &authority,
        &tampered,
        &ResumeRequest {
            expected_head: head.clone(),
            tick: 3,
            live_workers: BTreeSet::from(["worker-a".to_owned()]),
            target: target.clone(),
        },
    )
    .unwrap_err();
    assert_eq!(error, ProductError::AuthorityInvalid);
    let surfaces = format!(
        "{authority:?}\n{permit:?}\n{}\n{}",
        serde_json::to_string(&permit).unwrap(),
        error
    );
    assert!(!surfaces.contains(std::str::from_utf8(SECRET_CANARY).unwrap()));
    let wrong_root =
        root_authority_for_test(Actor::parse("not-the-root").unwrap(), SECRET_CANARY).unwrap();
    let wrong_root_permit = issue_action_permit_for_test(
        &wrong_root,
        RootActionPermitIssuance {
            operation: RootOperation::Resume,
            binding: binding(),
            workspace_identity: workspace.identity(),
            journal_head_identity: &journal_head_identity(&head).unwrap(),
            issued_tick: 2,
            expires_tick: 10,
            nonce: b"wrong-root-nonce-0123456789",
            target: target.clone(),
        },
    )
    .unwrap();
    assert_eq!(
        resume(
            &context(),
            &workspace,
            &wrong_root,
            &wrong_root_permit,
            &ResumeRequest {
                expected_head: head,
                tick: 3,
                live_workers: BTreeSet::from(["worker-a".to_owned()]),
                target,
            },
        )
        .unwrap_err(),
        ProductError::AuthorityInvalid
    );
    assert_eq!(recursive_fingerprint(root.path()), before);
}

#[test]
fn stale_candidate_and_result_substitution_never_write() {
    let (root, head, commitment) = submitted("stale-substitution");
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let before = recursive_fingerprint(root.path());
    let stale_context = ProductContext::new(
        graph(),
        policy(),
        Binding::new(&digest('c'), &digest('d')).unwrap(),
        root_actor(),
    );
    assert_eq!(
        query(
            &stale_context,
            &workspace,
            &QueryRequest {
                expected_head: head.clone(),
                tick: 4,
                live_workers: BTreeSet::from(["worker-a".to_owned()]),
            },
        )
        .unwrap_err(),
        ProductError::StaleCandidate
    );
    let authority = authority();
    let target = PermitTarget {
        lease_id: Some("lease-001".to_owned()),
        result_commitment_id: Some(digest('e')),
        ..PermitTarget::default()
    };
    let permit = permit(
        &authority,
        RootOperation::Resume,
        &workspace,
        &head,
        4,
        target.clone(),
    );
    assert_ne!(target.result_commitment_id.as_ref(), Some(&commitment));
    assert_eq!(
        resume(
            &context(),
            &workspace,
            &authority,
            &permit,
            &ResumeRequest {
                expected_head: head,
                tick: 4,
                live_workers: BTreeSet::from(["worker-a".to_owned()]),
                target,
            },
        )
        .unwrap_err(),
        ProductError::ResultSubstitution
    );
    assert_eq!(recursive_fingerprint(root.path()), before);
}
