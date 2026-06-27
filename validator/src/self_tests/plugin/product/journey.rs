use serde_json::{Value, json};

#[test]
fn plugin_product_flow_reports_missing_schema_entry_edges_and_categories() {
    let root = crate::self_tests::boundaries::support::temp_root("plugin-product-flow");
    std::fs::create_dir_all(&root).expect("root");
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({
            "skills":[{"path":"skills/fit-repo/SKILL.md"}],
            "schemas":["schemas/fit-repo-receipt.schema.json"],
            "authorable_templates":["templates/PRODUCT_FITNESS.md"],
            "agents":[{"path":"custom-agents/harness-product-simplicity-falsifier.toml"}]
        }))
        .expect("manifest"),
    )
    .expect("write manifest");
    std::fs::create_dir_all(root.join("templates/agent-standards")).expect("standards dir");
    std::fs::write(
        root.join("templates/agent-standards/enforcement.json"),
        serde_json::to_vec(&json!({"rows":[{"id":"missing-standard-row"}]})).expect("standards"),
    )
    .expect("write standards");
    let flow = json!({
        "schema": "wrong",
        "entrypoints": [],
        "edges": [],
        "required_surfaces": ["missing/setup.md"],
        "skills": [],
        "schemas": [],
        "templates": [],
        "custom_agents": [],
        "standards_rows": []
    });
    let failures = crate::audit::plugin::product::cohesion::flow_value_failures(&root, &flow);
    for expected in [
        "plugin_flow_manifest_malformed:schema",
        "plugin_flow_entrypoint_missing",
        "plugin_flow_required_edge_missing",
        "plugin_flow_setup_file_not_packaged:missing/setup.md",
        "plugin_flow_category_missing:setup_scripts",
        "plugin_flow_manifest_path_missing:skills:skills/fit-repo/SKILL.md",
        "plugin_flow_manifest_path_missing:schemas:schemas/fit-repo-receipt.schema.json",
        "plugin_flow_manifest_path_missing:templates:templates/PRODUCT_FITNESS.md",
        "plugin_flow_manifest_path_missing:custom_agents:custom-agents/harness-product-simplicity-falsifier.toml",
        "plugin_flow_standards_row_missing:missing-standard-row",
    ] {
        assert!(
            failures.iter().any(|item| item == expected),
            "{expected}: {failures:?}"
        );
    }
    std::fs::remove_dir_all(root).expect("cleanup flow");
}

#[test]
fn plugin_product_visible_entry_and_receipt_adapters_are_typed() {
    assert_eq!(
        crate::audit::plugin::product::cohesion::plugin_json_failures(&json!({
            "interface": {"defaultPrompt": ["Use harness-ultragoal:fit-repo first."]}
        })),
        Vec::<String>::new()
    );
    assert_eq!(
        crate::audit::plugin::product::cohesion::plugin_json_failures(&json!({"interface":{}})),
        vec!["plugin_flow_entrypoint_not_visible".to_string()]
    );

    let root = crate::self_tests::boundaries::support::repo_root();
    let fit = crate::json_boundary::read_json(
        &root.join("validation_artifacts/harness/fit-repo-receipt.json"),
    )
    .expect("fit receipt");
    let journey = crate::json_boundary::read_json(
        &root.join("validation_artifacts/harness/plugin-product-journey-receipt.json"),
    )
    .expect("journey receipt");
    let fit_failures =
        crate::audit::plugin::product::cohesion::fit_receipt_value_failures(&root, &fit);
    assert!(fit_failures.is_empty(), "{fit_failures:?}");
    assert!(
        crate::audit::plugin::product::cohesion::journey_value_failures(&root, &journey).is_empty()
    );
}

#[test]
fn plugin_product_journey_rejects_empty_and_malformed_fields() {
    let root = crate::self_tests::boundaries::support::temp_root("plugin-product-journey");
    std::fs::create_dir_all(&root).expect("root");
    let failures =
        crate::audit::plugin::product::cohesion::journey_value_failures(&root, &Value::Null);
    assert!(failures.contains(&"plugin_product_journey_incomplete".to_string()));
    std::fs::remove_dir_all(root).expect("cleanup journey");
}

#[test]
fn plugin_flow_authority_requires_checks_edges_and_completion_receipts() {
    let flow = json!({
        "validator_checks":["source-package-inventory"],
        "edges":[{"from":"harness-ultragoal:fit-repo","to":"templates/.harness/coverage-manifest.json"}],
        "flows":[
            {"id":"setup","required_edges":["missing->edge"]},
            {
                "id":"complete",
                "completion_receipt":"validation_artifacts/harness/plugin-product-journey-receipt.json",
                "required_edges":[]
            }
        ]
    });
    let failures = crate::audit::plugin::flow::authority::failures(&flow);
    assert!(
        failures.iter().any(|item| {
            item == "plugin_flow_validator_check_missing:agent-standards-enforcement"
        })
    );
    assert!(failures.iter().any(|item| {
        item == "plugin_flow_required_edge_missing:harness-ultragoal:fit-repo->validation_artifacts/harness/fit-repo-receipt.json"
    }));
    assert!(
        failures
            .iter()
            .any(|item| item == "plugin_flow_completion_receipt_missing:setup"),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|item| item == "plugin_flow_required_edge_missing:setup:missing->edge"),
        "{failures:?}"
    );

    let no_flows = json!({"validator_checks":[],"edges":[]});
    assert!(
        crate::audit::plugin::flow::authority::failures(&no_flows)
            .iter()
            .any(|item| item == "plugin_flow_completion_receipt_missing:flows")
    );
}
