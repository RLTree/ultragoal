#[test]
fn active_lease_issuance_rejects_identity_and_authority_substitution() {
    let older_ancestor = source_repo("lease-valid-ancestor-substitution");
    mutate_registry(&older_ancestor, |registry| {
        activate_n11_lease(&older_ancestor, registry);
        for gate in registry["prelaunch_gates"].as_array_mut().unwrap() {
            if gate["status"] == "current" {
                gate["observed_source_base"]["commit"] =
                    "97e24c9706e7b489bdbdc6184ff9520a7116c6fd".into();
                gate["observed_source_base"]["tree"] =
                    "c1d0cc65ffce60e4d917e14cb8b9ac4664d71a3e".into();
            }
        }
        for record in registry["lease_state"]["active_records"]
            .as_array_mut()
            .unwrap()
        {
            record["base_commit"] = "97e24c9706e7b489bdbdc6184ff9520a7116c6fd".into();
            record["base_tree"] = "c1d0cc65ffce60e4d917e14cb8b9ac4664d71a3e".into();
        }
    });
    assert_inventory_error(
        &older_ancestor,
        "prelaunch gate operation identity does not bind source base",
    );

    let rebound_ancestor = source_repo("lease-coherent-ancestor-substitution");
    mutate_registry(&rebound_ancestor, |registry| {
        activate_n11_lease(&rebound_ancestor, registry);
        let commit = "97e24c9706e7b489bdbdc6184ff9520a7116c6fd";
        let tree = "c1d0cc65ffce60e4d917e14cb8b9ac4664d71a3e";
        for gate in registry["prelaunch_gates"].as_array_mut().unwrap() {
            if gate["status"] == "current" {
                let id = gate["id"].as_str().unwrap().to_owned();
                gate["observed_source_base"]["commit"] = commit.into();
                gate["observed_source_base"]["tree"] = tree.into();
                gate["operation_id"] = format!("prelaunch-{id}-{commit}-{tree}").into();
            }
        }
        for record in registry["lease_state"]["active_records"]
            .as_array_mut()
            .unwrap()
        {
            record["base_commit"] = commit.into();
            record["base_tree"] = tree.into();
        }
    });
    assert_inventory_error(
        &rebound_ancestor,
        "lease source base is not the root-issued base",
    );

    let record_mismatch = source_repo("lease-record-base-mismatch");
    mutate_registry(&record_mismatch, |registry| {
        activate_n11_lease(&record_mismatch, registry);
        registry["lease_state"]["active_records"][0]["base_tree"] =
            "0000000000000000000000000000000000000000".into();
    });
    assert_inventory_error(
        &record_mismatch,
        "active lease base differs from source base",
    );

    let duplicate_gate = source_repo("lease-duplicate-gate");
    mutate_registry(&duplicate_gate, |registry| {
        activate_n11_lease(&duplicate_gate, registry);
        let duplicate = registry["prelaunch_gates"][0].clone();
        registry["prelaunch_gates"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
    });
    assert_inventory_error(&duplicate_gate, "required gate is missing or duplicated");

    let scope_substitution = source_repo("lease-scope-substitution");
    mutate_registry(&scope_substitution, |registry| {
        activate_n11_lease(&scope_substitution, registry);
        registry["lease_state"]["active_records"][0]["owned_symbols"] =
            serde_json::json!(["ultragoal::distribution"]);
    });
    assert_inventory_error(
        &scope_substitution,
        "lease owned_symbols differs from scope authority",
    );

    let downgraded_frontier = source_repo("lease-frontier-downgrade");
    mutate_registry(&downgraded_frontier, |registry| {
        activate_n11_lease(&downgraded_frontier, registry);
        registry["pre_adoption_source"]["frontier"] = "FORGED_FRONTIER".into();
    });
    assert_inventory_error(&downgraded_frontier, "scheduler frontier is unknown");

    let duplicate_scope = source_repo("lease-duplicate-scope-id");
    mutate_registry(&duplicate_scope, |registry| {
        activate_n11_lease(&duplicate_scope, registry);
        let duplicate = registry["scope_mappings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|scope| scope["scope_id"] == "WS-EVAL")
            .unwrap()
            .clone();
        registry["scope_mappings"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
    });
    assert_inventory_error(&duplicate_scope, "duplicate scope authority ID");
}
