use serde_json::json;

#[test]
fn schema_rule_contracts_cover_fixture_receipt_and_keyword_boundaries() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let store = crate::schema_catalog::load(&root);

    let red_packet_errors =
        crate::schema_catalog::schema_errors(&store, "red-packet.schema.json", &json!({}));
    assert!(
        red_packet_errors
            .iter()
            .any(|err| err == "expected_failure is required")
    );
    assert!(
        red_packet_errors
            .iter()
            .any(|err| err == "json_patch is required")
    );

    let duplicate_catalog = crate::schema_catalog::schema_errors(
        &store,
        "red-fixtures-catalog.schema.json",
        &json!([{"id":"dup"},{"id":"dup"}]),
    );
    assert!(
        duplicate_catalog
            .iter()
            .any(|err| err == "red catalog id uniqueness mismatch")
    );
    let object_catalog = crate::schema_catalog::schema_errors(
        &store,
        "red-fixtures-catalog.schema.json",
        &json!({}),
    );
    assert!(
        object_catalog
            .iter()
            .any(|err| err == "red catalog must be an array")
    );

    let validator_receipt = json!({
        "commit":"not-a-digest",
        "target_revision":{"kind":"package_digest","value":"not-a-digest"},
        "required_check_ids":["dup","dup"],
        "required_red_fixture_ids":["red"],
        "red_fixtures":{"red":{}},
        "required_execplan_refs":["wrong-plan.md"]
    });
    let validator_errors = crate::schema_catalog::schema_errors(
        &store,
        "validator-receipt.schema.json",
        &validator_receipt,
    );
    assert!(
        validator_errors
            .iter()
            .any(|err| err == "required_check_ids contains duplicate values")
    );
    assert!(
        validator_errors
            .iter()
            .any(|err| err.contains("required_check_ids expected"))
    );
    assert!(
        validator_errors
            .iter()
            .any(|err| err == "required_execplan_refs must name all active ExecPlans")
    );

    let target_errors =
        crate::schema_catalog::schema_errors(&store, "target-repo-receipt.schema.json", &json!({}));
    assert!(target_errors.iter().any(|err| err == "schema is required"));

    assert_eq!(
        crate::schema_catalog::schema_error_code(&[
            "$.plugin_manifest.agents[0].path: const mismatch".to_string()
        ]),
        "plugin_agent_path_missing"
    );
    assert_eq!(
        crate::schema_catalog::schema_error_code(&[
            "semantic_classification_receipts[0].classifier_evidence: missing required".to_string()
        ]),
        "semantic_classification_receipt_malformed"
    );
}

#[test]
fn schema_catalog_loader_reports_invalid_rows_and_catalog_path_errors() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("schema-catalog-loader");
    std::fs::create_dir_all(root.join("schemas")).expect("schemas");
    std::fs::write(
        root.join("schemas/schema-catalog.json"),
        serde_json::to_vec(&json!({"schemas":[{"id":"missing-path"},{"path":"missing-id.json"}]}))
            .expect("catalog json"),
    )
    .expect("write schema catalog");
    let store = crate::schema_catalog::load(&root);
    assert!(
        store
            .errors
            .iter()
            .any(|err| err.contains("schema row invalid")),
        "{:?}",
        store.errors
    );
    std::fs::remove_dir_all(&root).expect("cleanup schema catalog rows");

    #[cfg(unix)]
    {
        let root =
            crate::self_tests::boundaries::workspace_fixtures::temp_root("schema-catalog-symlink");
        std::fs::create_dir_all(&root).expect("root");
        let outside =
            crate::self_tests::boundaries::workspace_fixtures::temp_root("schema-catalog-outside");
        std::fs::create_dir_all(&outside).expect("outside");
        std::os::unix::fs::symlink(&outside, root.join("schemas")).expect("schemas symlink");
        let store = crate::schema_catalog::load(&root);
        assert!(
            store
                .errors
                .iter()
                .any(|err| err.contains("schema-catalog: package path uses symlink")),
            "{:?}",
            store.errors
        );
        std::fs::remove_dir_all(root).expect("cleanup schema symlink root");
        std::fs::remove_dir_all(outside).expect("cleanup schema symlink outside");
    }
}

#[test]
fn fixture_bundle_rules_reject_claim_lane_backlog_and_manifest_gaps() {
    let fixture = json!({
        "completion_manifest":{
            "required_claim_ids":[],
            "claims":[
                {"id":"included-without-semantic","claim_ceiling_effect":"included"},
                {
                    "id":"bad-waiver",
                    "claim_ceiling_effect":"withheld",
                    "semantic_classification_receipts":[],
                    "product_cohesion_waiver":{"reason":"local_control_surface_only"}
                }
            ]
        },
        "lane_registry":{"root_verification_phases":{"pre_merge_lane_gate":{}}},
        "validator_receipt":{"schema":"harness-ultragoal.validator-receipt.v1"},
        "verification_backlog":{"rows":[{}]},
        "plugin_manifest":{"skills":[],"agents":[]}
    });
    let errors = crate::schema_catalog::fixture_schema_rules::fixture_bundle_errors(&fixture);
    for expected in [
        "required_claim_ids must be non-empty",
        "claims[0].semantic_classification_receipts is required",
        "claims[1].product_cohesion_waiver.reason is not allowed",
        "root_verification_phases.post_merge_integration_gate is required",
        "root_verification_phases.final_all_lanes_gate is required",
        "verification_backlog.rows[0].attempts is required",
        "plugin_manifest.skills missing required agent-first-repo-init",
        "plugin_manifest.agents missing required harness-contract-claim-falsifier",
    ] {
        assert!(
            errors.iter().any(|err| err == expected),
            "{expected}: {errors:?}"
        );
    }
    let mut complete_manifest = fixture.clone();
    complete_manifest["plugin_manifest"] = json!({
        "skills": crate::audit::contract::REQUIRED_SKILLS
            .iter()
            .map(|name| json!({"name": name}))
            .collect::<Vec<_>>(),
        "agents": crate::audit::contract::REQUIRED_AGENTS
            .iter()
            .map(|name| json!({"name": name}))
            .collect::<Vec<_>>()
    });
    let complete_errors =
        crate::schema_catalog::fixture_schema_rules::fixture_bundle_errors(&complete_manifest);
    assert!(
        !complete_errors
            .iter()
            .any(|err| err.contains("plugin_manifest.skills missing required")),
        "{complete_errors:?}"
    );
    assert!(
        !complete_errors
            .iter()
            .any(|err| err.contains("plugin_manifest.agents missing required")),
        "{complete_errors:?}"
    );
    assert!(
        crate::schema_catalog::fixture_schema_rules::red_catalog_errors(&json!([
            {"id":"one"},
            {"id":"two"}
        ]))
        .is_empty()
    );
}
