use serde_json::{Value, json};

#[test]
fn plugin_product_flow_reports_missing_schema_entry_edges_and_categories() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("plugin-product-flow");
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
    let failures =
        crate::audit::plugin::product::cohesion::flow_manifest_projection_failures(&root, &flow);
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

    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let fit_repo_receipt = crate::json_boundary::read_json(
        &root.join("validation_artifacts/harness/fit-repo-receipt.json"),
    )
    .expect("fit-repo receipt");
    let journey = with_current_evidence_digests(
        &root,
        crate::json_boundary::read_json(
            &root.join("validation_artifacts/harness/plugin-product-journey-receipt.json"),
        )
        .expect("journey receipt"),
    );
    let target = fit_repo_receipt
        .pointer("/target_revision/value")
        .and_then(serde_json::Value::as_str)
        .expect("fit-repo target digest");
    let fit_repo_failures =
        crate::audit::plugin::product::cohesion::fit_repo_receipt_value_failures_with_candidate(
            &root,
            &fit_repo_receipt,
            target,
        );
    assert!(fit_repo_failures.is_empty(), "{fit_repo_failures:?}");
    let journey_target = journey
        .pointer("/target_revision/value")
        .and_then(serde_json::Value::as_str)
        .expect("journey target digest");
    assert!(
        crate::audit::plugin::product::cohesion::journey_value_failures_with_candidate(
            &root,
            &journey,
            journey_target
        )
        .is_empty()
    );
}

fn with_current_evidence_digests(root: &std::path::Path, mut value: Value) -> Value {
    for item in value
        .get_mut("evidence")
        .and_then(Value::as_array_mut)
        .into_iter()
        .flatten()
    {
        rebind_item_digest(root, item);
    }
    if let Some(item) = value.get_mut("error_path_evidence") {
        rebind_item_digest(root, item);
    }
    value
}

fn rebind_item_digest(root: &std::path::Path, item: &mut Value) {
    let Some(path) = item.get("path").and_then(Value::as_str) else {
        return;
    };
    if let Ok(digest) = crate::digest::file(&root.join(path)) {
        item["digest"] = json!(digest);
    }
}

#[test]
fn plugin_product_journey_digest_rebind_skips_items_without_paths() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "plugin-product-journey-rebind",
    );
    std::fs::create_dir_all(&root).expect("journey rebind root");
    let value = with_current_evidence_digests(
        &root,
        json!({
            "evidence": [{"digest": "unchanged"}],
            "error_path_evidence": {"digest": "still-unchanged"}
        }),
    );
    assert_eq!(value["evidence"][0]["digest"], "unchanged");
    assert_eq!(value["error_path_evidence"]["digest"], "still-unchanged");
    std::fs::remove_dir_all(root).expect("cleanup journey rebind");
}

#[test]
fn plugin_product_journey_rejects_empty_and_malformed_fields() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("plugin-product-journey");
    std::fs::create_dir_all(&root).expect("root");
    let failures =
        crate::audit::plugin::product::cohesion::journey_value_failures(&root, &Value::Null);
    assert!(failures.contains(&"plugin_product_journey_incomplete".to_string()));
    std::fs::remove_dir_all(root).expect("cleanup journey");
}

#[test]
fn plugin_product_journey_requires_same_candidate_typed_authority() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let current = crate::package::inventory::package_digest(&root).expect("package digest");
    let mut journey = crate::json_boundary::read_json(
        &root.join("validation_artifacts/harness/plugin-product-journey-receipt.json"),
    )
    .expect("journey receipt");

    let mut missing_authority = journey.clone();
    missing_authority.as_object_mut().unwrap().remove("status");
    missing_authority
        .as_object_mut()
        .unwrap()
        .remove("target_revision");
    let failures =
        crate::audit::plugin::product::cohesion::journey_value_failures(&root, &missing_authority);
    assert!(failures.contains(&"plugin_product_journey_status_not_pass".to_string()));
    assert!(
        failures.contains(&"plugin_product_journey_target_revision_not_package_digest".to_string())
    );
    assert!(failures.contains(&"plugin_product_journey_target_digest_mismatch".to_string()));

    journey["status"] = json!("fail");
    journey["target_revision"] = json!({"kind":"source_tree_digest","value":current});
    let failures = crate::audit::plugin::product::cohesion::journey_value_failures(&root, &journey);
    assert!(failures.contains(&"plugin_product_journey_status_not_pass".to_string()));
    assert!(
        failures.contains(&"plugin_product_journey_target_revision_not_package_digest".to_string())
    );

    journey["status"] = json!("pass");
    journey["target_revision"] = json!({"kind":"package_digest","value":crate::self_tests::boundaries::workspace_fixtures::sha('9')});
    let failures = crate::audit::plugin::product::cohesion::journey_value_failures(&root, &journey);
    assert!(failures.contains(&"plugin_product_journey_target_digest_mismatch".to_string()));
}

#[test]
fn plugin_flow_authority_requires_checks_edges_and_completion_receipts() {
    let flow = json!({
        "validator_checks":["source-package-inventory"],
        "edges":[{"from":"harness-ultragoal:fit-repo","to":"templates/.harness/coverage-manifest.json"}],
        "flows":[
            {"required_edges":[]},
            {"id":"setup","required_edges":["missing->edge"]},
            {
                "id":"complete",
                "completion_receipt":"validation_artifacts/harness/plugin-product-journey-receipt.json",
                "required_edges":[]
            }
        ]
    });
    let failures = crate::audit::plugin::flow::authority::failures_from_value(&flow);
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
            .any(|item| item == "plugin_flow_completion_receipt_missing:<unknown>"),
        "{failures:?}"
    );
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
        crate::audit::plugin::flow::authority::failures_from_value(&no_flows)
            .iter()
            .any(|item| item == "plugin_flow_completion_receipt_missing:flows")
    );
}
