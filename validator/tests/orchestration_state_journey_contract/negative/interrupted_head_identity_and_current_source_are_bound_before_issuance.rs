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
