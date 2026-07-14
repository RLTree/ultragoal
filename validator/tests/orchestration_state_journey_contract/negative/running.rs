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
