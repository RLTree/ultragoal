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
            RootActionPermitIssuance {
                operation: RootOperation::Reconcile,
                binding: binding(),
                workspace_identity: workspace.identity(),
                journal_head_identity: &journal_head_identity(&head).unwrap(),
                issued_tick: 3,
                expires_tick: 14,
                nonce: b"action-only-reconcile-nonce-0123",
                target: target(),
            },
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
