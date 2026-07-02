use serde_json::json;

#[test]
fn operating_loop_rejects_row_shape_research_inputs() {
    let root = crate::self_tests::boundaries::support::temp_root("observe-research-inputs");
    let mut inventory = super::super::support::fitted_inventory();
    inventory["operating_loop"]["research_inputs"]
        .as_array_mut()
        .unwrap()
        .retain(|row| {
            row.get("source_id").and_then(serde_json::Value::as_str)
                != Some("agentic-gold-standard-stack-synthesis-2026-07-01")
        });
    inventory["operating_loop"]["research_inputs"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "source_id": "unknown-source",
            "url": "https://example.test/unknown",
            "repo_rule": "unknown source must fail"
        }));
    inventory["operating_loop"]["research_inputs"][0]["url"] = json!("https://wrong.example");
    inventory["operating_loop"]["research_inputs"][1]
        .as_object_mut()
        .unwrap()
        .remove("repo_rule");
    inventory["operating_loop"]["research_inputs"]
        .as_array_mut()
        .unwrap()
        .push(json!({"url":"https://example.test/missing-id"}));
    let duplicate = inventory["operating_loop"]["research_inputs"][0].clone();
    inventory["operating_loop"]["research_inputs"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);

    let mut failures = Vec::new();
    super::super::super::operating::check(&root, &inventory, &mut failures);
    assert!(
        failures.contains(
            &"observability_operating_research_input_missing:agentic-gold-standard-stack-synthesis-2026-07-01"
                .to_string()
        )
    );
    assert!(
        failures
            .contains(&"observability_operating_research_input_unknown:unknown-source".to_string())
    );
    assert!(
        failures.contains(
            &"observability_operating_research_input_url_mismatch:openai-harness-engineering"
                .to_string()
        )
    );
    assert!(failures.contains(
        &"observability_operating_research_input_rule_missing:openai-codex-iterative-repair-loops"
            .to_string()
    ));
    assert!(
        failures.contains(&"observability_operating_research_input_source_id_missing".to_string())
    );
    assert!(failures.contains(
        &"observability_operating_research_input_duplicate:openai-harness-engineering".to_string()
    ));
    let _ = std::fs::remove_dir_all(root);
}
