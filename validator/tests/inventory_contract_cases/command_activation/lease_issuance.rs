#[test]
fn active_lease_issuance_rejects_identity_and_authority_substitution() {
    let older_ancestor = source_repo("lease-valid-ancestor-substitution");
    mutate_registry(&older_ancestor, |registry| {
        activate_n08_lease(registry);
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
        "lease source base is not the root-issued base",
    );

    let record_mismatch = source_repo("lease-record-base-mismatch");
    mutate_registry(&record_mismatch, |registry| {
        activate_n08_lease(registry);
        registry["lease_state"]["active_records"][0]["base_tree"] =
            "0000000000000000000000000000000000000000".into();
    });
    assert_inventory_error(
        &record_mismatch,
        "active lease base differs from source base",
    );

    let duplicate_gate = source_repo("lease-duplicate-gate");
    mutate_registry(&duplicate_gate, |registry| {
        activate_n08_lease(registry);
        let duplicate = registry["prelaunch_gates"][0].clone();
        registry["prelaunch_gates"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
    });
    assert_inventory_error(&duplicate_gate, "required gate is missing or duplicated");

    let scope_substitution = source_repo("lease-scope-substitution");
    mutate_registry(&scope_substitution, |registry| {
        activate_n08_lease(registry);
        registry["lease_state"]["active_records"][0]["owned_symbols"] =
            serde_json::json!(["ultragoal::distribution"]);
    });
    assert_inventory_error(
        &scope_substitution,
        "lease owned_symbols differs from scope authority",
    );

    let downgraded_frontier = source_repo("lease-frontier-downgrade");
    mutate_registry(&downgraded_frontier, |registry| {
        activate_n08_lease(registry);
        registry["pre_adoption_source"]["frontier"] = "FORGED_FRONTIER".into();
    });
    assert_inventory_error(&downgraded_frontier, "scheduler frontier is unknown");

    let duplicate_scope = source_repo("lease-duplicate-scope-id");
    mutate_registry(&duplicate_scope, |registry| {
        activate_n08_lease(registry);
        let duplicate = registry["scope_mappings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|scope| scope["scope_id"] == "WS-FIT")
            .unwrap()
            .clone();
        registry["scope_mappings"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
    });
    assert_inventory_error(&duplicate_scope, "duplicate scope authority ID");
}

fn activate_n08_lease(registry: &mut serde_json::Value) {
    registry["pre_adoption_source"]["frontier"] = "N08_READY_N09_INTEGRATED_SOURCE_FRONTIER".into();
    registry["pre_adoption_source"]["eligible_scheduler_nodes"] = serde_json::json!(["N08"]);

    let lanes = registry["lanes"].as_array_mut().unwrap();
    let n08 = lanes.iter_mut().find(|lane| lane["id"] == "N08").unwrap();
    n08["state"] = "ready".into();
    n08["current_identity"] = serde_json::Value::Null;
    n08["ceiling"] = "adopted_reobservation_required".into();
    let lane = n08.clone();

    let scope = registry["scope_mappings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|scope| scope["scope_id"] == "WS-PLUGIN")
        .unwrap()
        .clone();
    let dependency_identities = lane["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| {
            registry["lanes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|candidate| candidate["id"] == *id)
                .unwrap()["current_identity"]
                .clone()
        })
        .collect::<Vec<_>>();

    let mut record = registry["lease_state"]["record_template"].clone();
    record["lease_id"] = "LEASE-TEST-N08".into();
    record["lane_id"] = "N08".into();
    record["owner"] = lane["owner"].clone();
    record["scope_ids"] = lane["scope_ids"].clone();
    record["branch"] = "codex/test-n08".into();
    record["worktree"] = "/tmp/test-n08".into();
    record["owned_files"] = scope["owned_roots"].clone();
    record["owned_symbols"] = scope["owned_symbols"].clone();
    record["generated_outputs"] = scope["generated_roots"].clone();
    record["fixtures"] = scope["fixture_roots"].clone();
    record["effects"] = scope["effects"].clone();
    record["consumed_set"]["dependency_identities"] =
        serde_json::Value::Array(dependency_identities);
    record["invalidated_by"] = lane["invalidation_contract"]["invalidated_by"].clone();
    record["status"] = "issued".into();
    record["clean_handoff"] = "pending".into();
    record["reachable_tip"] = true.into();

    registry["lease_state"]["status"] = "active".into();
    registry["lease_state"]["active_records"] = serde_json::json!([record]);
}

fn mutate_registry(repo: &TestRepo, mutate: impl FnOnce(&mut serde_json::Value)) {
    let path = repo.root.join("LANE_REGISTRY.json");
    let mut registry = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    mutate(&mut registry);
    fs::write(path, serde_json::to_vec(&registry).unwrap()).unwrap();
}
