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
fn research_registry_rejects_missing_stale_or_uninventoried_source_corpus() {
    let (_root, mut cards, mut registry, trace) = repo_values();
    cards["sources"][0]["source_corpus_path"] =
        json!("artifacts/source-snapshots/missing-source-corpus.txt");
    registry["sources"][0]["source_corpus_path"] =
        json!("artifacts/source-snapshots/missing-source-corpus.txt");
    registry["sources"][0]["source_corpus_digest"] =
        json!(crate::self_tests::boundaries::support::sha('1'));
    let failures = failures(&cards, &registry, &trace);
    for expected in [
        "research_registry_source_corpus_digest_stale:openai-harness-engineering",
        "research_source_corpus_package_omitted:openai-harness-engineering:artifacts/source-snapshots/missing-source-corpus.txt",
        "research_source_corpus_missing:openai-harness-engineering:artifacts/source-snapshots/missing-source-corpus.txt",
    ] {
        assert!(
            failures.iter().any(|item| item == expected),
            "{expected}\n{failures:#?}"
        );
    }
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
fn research_trace_rejects_missing_fixture_and_mapping_edges() {
    let (_root, cards, registry, mut trace) = repo_values();
    trace["entries"][0]["red_fixture_ids"] = json!([]);
    trace["entries"][0]["tamper_fixture_ids"] = json!([]);
    trace["entries"][0]["green_fixture_paths"] = json!([]);
    trace["entries"][0]["setup_retrofit_outputs"] = json!([]);
    let failures = failures(&cards, &registry, &trace);
    for expected in [
        "research_trace_missing_red_fixture_ids:openai-harness-agent-legible-repo",
        "research_trace_missing_tamper_fixture_ids:openai-harness-agent-legible-repo",
        "research_trace_missing_green_fixture_paths:openai-harness-agent-legible-repo",
        "research_trace_missing_setup_retrofit_outputs:openai-harness-agent-legible-repo",
    ] {
        assert!(
            failures.iter().any(|item| item == expected),
            "{expected}\n{failures:#?}"
        );
    }
}

#[test]
fn research_trace_rejects_unknown_tamper_fixture_id() {
    let (_root, cards, registry, mut trace) = repo_values();
    trace["entries"][0]["tamper_fixture_ids"] = json!(["not-a-real-tamper-fixture"]);
    let failures = failures(&cards, &registry, &trace);
    assert!(
        failures.iter().any(|item| {
            item
                == "research_trace_unknown_tamper_fixture:openai-harness-agent-legible-repo:not-a-real-tamper-fixture"
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
            row.get("source_id").and_then(Value::as_str)
                != Some("agentic-gold-standard-stack-synthesis-2026-07-01")
        });
    let failures = failures(&cards, &registry, &trace);
    assert!(
        failures.iter().any(|item| item
            == "research_registry_missing_required_source:agentic-gold-standard-stack-synthesis-2026-07-01"),
        "{failures:#?}"
    );
}

#[test]
fn research_registry_rejects_required_source_without_gate92_law() {
    let (_root, cards, mut registry, trace) = repo_values();
    registry["sources"][0]["canonical_law_ids_affected"] = json!([
        "research-source-authority-article-to-law-integration",
        "repo-knowledge-index-core-beliefs"
    ]);
    let failures = failures(&cards, &registry, &trace);
    assert!(
        failures
            .iter()
            .any(|item| item == "research_registry_gate92_law_missing:openai-harness-engineering"),
        "{failures:#?}"
    );
}

#[test]
fn research_trace_rejects_requirement_without_gate92_bindings() {
    let (_root, cards, registry, mut trace) = repo_values();
    trace["entries"][0]["canonical_law_ids"] = json!([
        "research-source-authority-article-to-law-integration",
        "repo-knowledge-index-core-beliefs"
    ]);
    trace["entries"][0]["schemas"] = json!(["schemas/research-article-to-law-trace.schema.json"]);
    trace["entries"][0]["claim_guards"] =
        json!(["research_backed_claims_withheld_without_current_article_to_law_trace"]);
    trace["entries"][0]["final_packet_fields"] = json!(["research_source_authority_status"]);
    trace["entries"][0]["update_goal_blockers"] = json!(["research_source_authority_incomplete"]);
    let failures = failures(&cards, &registry, &trace);
    assert!(
        failures.iter().any(|item| {
            item == "research_trace_gate92_canonical_law_ids_missing:openai-harness-agent-legible-repo"
        }),
        "{failures:#?}"
    );
    assert!(
        failures.iter().any(|item| {
            item == "research_trace_gate92_observability_schema_missing:openai-harness-agent-legible-repo"
        }),
        "{failures:#?}"
    );
    assert!(
        failures.iter().any(|item| {
            item == "research_trace_gate92_claim_guard_missing:openai-harness-agent-legible-repo"
        }),
        "{failures:#?}"
    );
    assert!(
        failures.iter().any(|item| {
            item == "research_trace_gate92_final_packet_field_missing:openai-harness-agent-legible-repo"
        }),
        "{failures:#?}"
    );
    assert!(
        failures.iter().any(|item| {
            item == "research_trace_gate92_update_goal_blocker_missing:openai-harness-agent-legible-repo"
        }),
        "{failures:#?}"
    );
}

#[test]
fn research_registry_rejects_unanchored_card_and_registry_orphan_edges() {
    let (_root, mut cards, mut registry, trace) = repo_values();
    cards["sources"][0]["requirements"][0]["source_evidence_ids"] = json!(["missing-anchor"]);
    registry["sources"][0]["source_id"] = json!("orphan-source");
    let failures = failures(&cards, &registry, &trace);
    assert!(
        failures
            .iter()
            .any(|item| { item.starts_with("research_source_card_requirement_unknown_anchor:") })
    );
    assert!(
        failures
            .iter()
            .any(|item| { item == "research_registry_source_card_missing:orphan-source" })
    );
    assert!(
        failures
            .iter()
            .any(|item| { item == "research_registry_source_artifact_missing:orphan-source" })
    );
}
