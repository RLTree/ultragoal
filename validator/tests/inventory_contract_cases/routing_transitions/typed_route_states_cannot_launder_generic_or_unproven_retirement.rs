#[test]
fn typed_route_states_cannot_launder_generic_or_unproven_retirement() {
    let repo = TestRepo::new("route-demotion-overclaim");
    let path = repo.root.join("migration/authority-routes.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let transition = &mut value["routes"][0]["transition"];
    transition["compatibility_behavior"] = serde_json::json!("verified");
    transition["compatibility_boundary"] = serde_json::json!("adopted");
    transition["replacement_state"] = serde_json::json!("verified");
    transition["active_reader_writer_state"] = serde_json::json!("none-verified");
    transition["observed_authority_state"] = serde_json::json!("context-only");
    transition["equivalence_proof"] = serde_json::json!("verified");
    transition["physical_cleanup_state"] = serde_json::json!("preserve");
    transition["proof_refs"] = serde_json::json!(["evidence/route-proof.json"]);
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    repo.commit();

    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    let error = InventoryBuilder::new(&context).build().unwrap_err();
    assert!(error.to_string().contains("exact compiled proof"));
}

#[test]
fn od008_archive_tuple_cannot_be_partially_rewritten() {
    let repo = TestRepo::new("od008-archive-tuple");
    let path = repo.root.join("migration/authority-routes.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let route = value["routes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|route| route["route_id"] == "finalizer-ready-receipt-to-hct-claims")
        .unwrap();
    route["transition"]["replacement_state"] = serde_json::json!("verified");
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    repo.commit();

    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    let error = InventoryBuilder::new(&context).build().unwrap_err();
    assert!(error.to_string().contains("retired archive route"));
}

#[test]
fn od009_does_not_forbid_non_destructive_preservation() {
    let repo = TestRepo::new("route-preservation");
    let path = repo.root.join("migration/authority-routes.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["routes"][0]["transition"]["physical_cleanup_state"] = serde_json::json!("preserve");
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    repo.write(
        "docs/ultragoal-contract-2026-07/legacy.md",
        b"preserved legacy context\n",
    );
    repo.commit();

    let catalog = catalog(&repo);
    assert!(catalog.findings().iter().any(|finding| {
        finding.code == "parallel_authority"
            && finding.relative_path.as_deref() == Some("docs/ultragoal-contract-2026-07/legacy.md")
    }));
}

#[test]
fn closure_fixture_binds_open_obligation_and_fail_closed_dispositions() {
    let path = crate::repository_fixture::live_root()
        .join("fixtures/inventory-authority/closure-cases.json");
    let value: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let cases = value["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| (case["case_id"].as_str().unwrap(), case))
        .collect::<std::collections::BTreeMap<_, _>>();
    for (case_id, severity, disposition) in [
        (
            "sole-current-warning-open-obligation",
            "warning",
            "open-migration-obligation",
        ),
        (
            "compatibility-warning-open-obligation",
            "warning",
            "open-migration-obligation",
        ),
        ("sole-current-error-blocks", "error", "blocking"),
        ("compatibility-error-blocks", "error", "blocking"),
        ("unknown-warning-blocks", "warning", "blocking"),
        ("candidate-warning-blocks", "warning", "blocking"),
        (
            "zero-blocker-open-obligations-close",
            "warning",
            "closed-with-open-obligations",
        ),
    ] {
        let case = cases.get(case_id).unwrap();
        assert_eq!(case["severity"], severity);
        assert_eq!(case["closure_disposition"], disposition);
    }
}
