#[test]
fn active_lease_issuance_rejects_identity_and_authority_substitution() {
    let record_mismatch = source_repo("lease-record-base-mismatch");
    mutate_registry(&record_mismatch, |registry| {
        registry["lease_state"]["active_records"][0]["base_tree"] =
            "0000000000000000000000000000000000000000".into();
    });
    assert_inventory_error(
        &record_mismatch,
        "active lease base differs from observed source base",
    );

    let wrong_git_tree = source_repo("lease-git-tree-mismatch");
    mutate_registry(&wrong_git_tree, |registry| {
        for gate in registry["prelaunch_gates"].as_array_mut().unwrap() {
            if gate["status"] == "current" {
                gate["observed_source_base"]["tree"] =
                    "0000000000000000000000000000000000000000".into();
            }
        }
        for record in registry["lease_state"]["active_records"]
            .as_array_mut()
            .unwrap()
        {
            record["base_tree"] = "0000000000000000000000000000000000000000".into();
        }
    });
    assert_inventory_error(
        &wrong_git_tree,
        "observed source base has the wrong Git tree",
    );

    let duplicate_gate = source_repo("lease-duplicate-gate");
    mutate_registry(&duplicate_gate, |registry| {
        let duplicate = registry["prelaunch_gates"][0].clone();
        registry["prelaunch_gates"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
    });
    assert_inventory_error(
        &duplicate_gate,
        "required lease issuance gate is missing or duplicated",
    );

    let scope_substitution = source_repo("lease-scope-substitution");
    mutate_registry(&scope_substitution, |registry| {
        registry["lease_state"]["active_records"][1]["owned_symbols"] =
            serde_json::json!(["ultragoal::distribution"]);
    });
    assert_inventory_error(
        &scope_substitution,
        "active lease owned_symbols differ from scope authority",
    );
}

fn mutate_registry(repo: &TestRepo, mutate: impl FnOnce(&mut serde_json::Value)) {
    let path = repo.root.join("LANE_REGISTRY.json");
    let mut registry = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    mutate(&mut registry);
    fs::write(path, serde_json::to_vec(&registry).unwrap()).unwrap();
}
