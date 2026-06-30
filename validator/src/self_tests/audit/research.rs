use serde_json::{Value, json};

fn repo_values() -> (std::path::PathBuf, Value, Value, Value) {
    let root = crate::self_tests::boundaries::support::repo_root();
    let cards = crate::json_boundary::read_json(&root.join("docs/research-source-cards.json"))
        .expect("research source cards");
    let registry =
        crate::json_boundary::read_json(&root.join("docs/research-source-registry.json"))
            .expect("research source registry");
    let trace =
        crate::json_boundary::read_json(&root.join("docs/research-article-to-law-trace.json"))
            .expect("research article trace");
    (root, cards, registry, trace)
}

fn failures(cards: &Value, registry: &Value, trace: &Value) -> Vec<String> {
    let root = crate::self_tests::boundaries::support::repo_root();
    crate::audit::research::value_failures(&root, cards, registry, trace)
}

#[test]
fn research_registry_and_trace_accept_current_repo_mapping() {
    let (root, cards, registry, trace) = repo_values();
    let failures = crate::audit::research::value_failures(&root, &cards, &registry, &trace);
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn research_registry_rejects_stale_source_card_digest() {
    let (_root, cards, mut registry, trace) = repo_values();
    registry["sources"][0]["source_card_digest"] =
        json!(crate::self_tests::boundaries::support::sha('1'));
    let failures = failures(&cards, &registry, &trace);
    assert!(
        failures
            .iter()
            .any(|item| item
                == "research_registry_source_card_digest_stale:openai-harness-engineering"),
        "{failures:#?}"
    );
}

#[test]
fn research_trace_rejects_alias_only_mapping() {
    let (_root, cards, registry, mut trace) = repo_values();
    trace["entries"][0]["canonical_law_ids"] = json!(["HU-001"]);
    let failures = failures(&cards, &registry, &trace);
    assert!(
        failures
            .iter()
            .any(|item| item == "research_trace_alias_only:openai-harness-agent-legible-repo"),
        "{failures:#?}"
    );
}

#[test]
fn research_trace_rejects_missing_claim_guard_and_package_path() {
    let (_root, cards, registry, mut trace) = repo_values();
    trace["entries"][0]["claim_guards"] = json!([]);
    trace["entries"][0]["package_inventory_paths"] = json!(["docs/missing-research-proof.json"]);
    let failures = failures(&cards, &registry, &trace);
    assert!(
        failures.iter().any(|item| {
            item == "research_trace_missing_claim_guards:openai-harness-agent-legible-repo"
        }),
        "{failures:#?}"
    );
    assert!(
        failures.iter().any(|item| {
            item == "research_trace_package_omitted:openai-harness-agent-legible-repo:docs/missing-research-proof.json"
        }),
        "{failures:#?}"
    );
}

#[test]
fn research_registry_rejects_missing_mandatory_source() {
    let (_root, cards, mut registry, trace) = repo_values();
    registry["sources"]
        .as_array_mut()
        .expect("sources array")
        .retain(|row| {
            row.get("source_id").and_then(Value::as_str) != Some("openai-self-improving-tax-agent")
        });
    let failures = failures(&cards, &registry, &trace);
    assert!(
        failures.iter().any(|item| item
            == "research_registry_missing_required_source:openai-self-improving-tax-agent"),
        "{failures:#?}"
    );
}
