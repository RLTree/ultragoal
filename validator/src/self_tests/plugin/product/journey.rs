use serde_json::{Value, json};

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
