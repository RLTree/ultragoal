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
