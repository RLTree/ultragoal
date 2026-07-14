fn catalog(repo: &TestRepo) -> crate::inventory::AuthorityCatalog {
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    InventoryBuilder::new(&context).build().unwrap()
}

#[test]
fn live_exact_sources_are_sole_current_and_pending_migration() {
    let root = crate::repository_fixture::live_root();
    let context = LiveContext::build(inventory_request(&root)).unwrap();
    let catalog = InventoryBuilder::new(&context).build().unwrap();
    let pending = catalog
        .findings()
        .iter()
        .filter(|finding| finding.code == "sole_current_authority_pending_migration")
        .collect::<Vec<_>>();

    assert_eq!(pending.len(), 33);
    let serialized_before = catalog.to_canonical_json().unwrap();
    let closure = catalog.closure_status();
    assert_eq!(catalog.to_canonical_json().unwrap(), serialized_before);
    assert!(!closure.is_closed());
    assert_eq!(catalog.findings().len(), 81);
    assert_eq!(closure.blocker_count(), 34);
    assert_eq!(
        closure
            .open_obligations_by_code()
            .get("sole_current_authority_pending_migration"),
        Some(&33)
    );
    assert_eq!(closure.open_obligation_count(), 47);
    assert_eq!(
        closure.blockers_by_code(),
        &std::collections::BTreeMap::from([
            ("candidate_component_not_active".to_owned(), 10),
            ("missing_required_component".to_owned(), 9),
            ("parallel_authority".to_owned(), 14),
            ("projection_requires_canonical_reconciliation".to_owned(), 1,),
        ])
    );
    assert_eq!(
        closure.open_obligations_by_code(),
        &std::collections::BTreeMap::from([
            ("compatibility_route_retained".to_owned(), 14),
            ("sole_current_authority_pending_migration".to_owned(), 33),
        ])
    );
    assert!(
        !closure
            .blockers_by_code()
            .contains_key("sole_current_authority_pending_migration")
    );
    assert!(
        !catalog
            .findings()
            .iter()
            .any(|finding| finding.code == "pending_authority_verification_failed")
    );
    for finding in pending {
        let stable_id = finding.entry_id.as_deref().unwrap();
        let entry = catalog
            .entries()
            .iter()
            .find(|entry| entry.stable_id == stable_id)
            .unwrap();
        assert_eq!(entry.authority_state, AuthorityState::Legacy);
        assert_eq!(entry.active_status, ActiveStatus::Active);
        assert!(!catalog.findings().iter().any(|candidate| {
            candidate.code == "parallel_authority"
                && candidate.entry_id.as_deref() == Some(stable_id)
        }));
    }
}

#[test]
fn declared_routes_do_not_suppress_live_legacy_authority() {
    let repo = TestRepo::new("legacy-routes");
    repo.skill("ultragoal", "ultragoal");
    repo.write("plugin-manifest-draft.json", br#"{"legacy":true}"#);
    for (path, bytes) in [
        ("agents/old-agent.md", b"legacy agent\n".as_slice()),
        ("legacy/lane/owner.md", b"legacy lane\n".as_slice()),
        ("legacy/gate/rule.md", b"legacy gate\n".as_slice()),
        ("legacy/command-catalog.json", b"{}\n".as_slice()),
        ("legacy/finalizer.md", b"legacy finalizer\n".as_slice()),
        ("legacy/model.md", b"obsolete gpt-5.5 pin\n".as_slice()),
        (
            "docs/ultragoal-contract-2026-07/legacy.md",
            b"legacy contract\n".as_slice(),
        ),
    ] {
        repo.write(path, bytes);
    }
    repo.commit();

    let catalog = catalog(&repo);
    let legacy = catalog
        .entries()
        .iter()
        .filter(|entry| entry.authority_state == AuthorityState::Legacy)
        .collect::<Vec<_>>();
    assert!(legacy.len() >= 8);
    assert!(
        legacy
            .iter()
            .all(|entry| entry.active_status == ActiveStatus::Active)
    );
    let command_catalog = legacy
        .iter()
        .find(|entry| entry.relative_path == "legacy/command-catalog.json")
        .unwrap();
    assert!(catalog.findings().iter().any(|finding| {
        finding.code == "parallel_authority"
            && finding.entry_id.as_deref() == Some(command_catalog.stable_id.as_str())
    }));
    assert!(
        catalog
            .findings()
            .iter()
            .any(|finding| finding.code == "pending_authority_verification_failed")
    );
    assert!(
        !catalog
            .findings()
            .iter()
            .any(|finding| finding.code == "ambiguous_authority_route")
    );
    assert!(
        catalog
            .findings()
            .iter()
            .any(|finding| finding.code == "parallel_authority")
    );
}

#[test]
fn routing_registry_cannot_claim_cleanup_or_compatibility() {
    let repo = TestRepo::new("route-overclaim");
    let path = repo.root.join("migration/authority-routes.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["destructive_cleanup_authorized"] = serde_json::json!(true);
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    repo.commit();
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    assert!(
        InventoryBuilder::new(&context)
            .build()
            .unwrap_err()
            .to_string()
            .contains("OD-009")
    );

    let repo = TestRepo::new("route-compatibility-overclaim");
    let path = repo.root.join("migration/authority-routes.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["routes"][0]["transition"]["compatibility_behavior"] = serde_json::json!("verified");
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    repo.commit();
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    assert!(
        InventoryBuilder::new(&context)
            .build()
            .unwrap_err()
            .to_string()
            .contains("exact compiled proof")
    );
}

#[test]
fn routing_errors_do_not_echo_untrusted_identifiers() {
    let repo = TestRepo::new("route-canary");
    let path = repo.root.join("migration/authority-routes.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["routes"][0]["route_id"] = serde_json::json!("SECRET_CANARY");
    value["routes"][0]["canonical_target"] = serde_json::json!("SECRET_CANARY");
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    repo.commit();

    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    let error = InventoryBuilder::new(&context).build().unwrap_err();
    assert!(!error.to_string().contains("SECRET_CANARY"));
}
