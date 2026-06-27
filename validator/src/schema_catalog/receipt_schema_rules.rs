use serde_json::Value;
use std::collections::BTreeSet;

pub fn validator_receipt_errors(value: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    if !is_sha(value.get("commit").and_then(Value::as_str).unwrap_or("")) {
        errors.push("validator_receipt.commit must match sha256".to_string());
    }
    if value
        .pointer("/target_revision/kind")
        .and_then(Value::as_str)
        == Some("package_digest")
        && !is_sha(
            value
                .pointer("/target_revision/value")
                .and_then(Value::as_str)
                .unwrap_or(""),
        )
    {
        errors.push("validator_receipt.target_revision.value must match sha256".to_string());
    }
    unique_count(
        value,
        "required_check_ids",
        crate::audit::contract::CHECK_IDS.len(),
        &mut errors,
    );
    unique_count(
        value,
        "required_red_fixture_ids",
        red_fixture_count(value),
        &mut errors,
    );
    exact_execplan_refs(value, &mut errors);
    errors
}

fn red_fixture_count(value: &Value) -> usize {
    value
        .get("red_fixtures")
        .and_then(Value::as_object)
        .map_or(0, serde_json::Map::len)
}

fn exact_execplan_refs(value: &Value, errors: &mut Vec<String>) {
    let expected = [
        "harness-ultragoal-plans-and-orchestrator-automation-hardening.md",
        "mandatory-coverage-authority-and-enforcement.md",
        "mandatory-coverage-scope-authority-and-anti-theater.md",
        "mandatory-plugin-product-cohesion-and-fit-repo-authority.md",
        "mandatory-product-fitness-quality-in-use-enforcement.md",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let Some(items) = value
        .get("required_execplan_refs")
        .and_then(Value::as_array)
    else {
        errors.push("required_execplan_refs is required".to_string());
        return;
    };
    let observed = items
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    if observed != expected {
        errors.push("required_execplan_refs must name all active ExecPlans".to_string());
    }
}

pub fn target_receipt_schema_errors(value: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    for key in [
        "schema",
        "mode",
        "target_repo",
        "status",
        "checks",
        "command",
    ] {
        if value.get(key).is_none() {
            errors.push(format!("{key} is required"));
        }
    }
    errors
}

pub fn semantic_receipt_errors(value: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    for key in [
        "schema",
        "claim_id",
        "canonical_text_digest",
        "classifier_contract_id",
        "classifier_contract_version",
        "classifier_implementation_kind",
        "generated_at",
        "producer_actor_id",
        "classifier_actor_id",
        "actor_disjoint",
        "detected_semantic_classes",
        "rationale",
        "confidence",
        "ambiguity",
        "required_proof_gates",
        "claim_ceiling_recommendation",
        "receipt_digest",
    ] {
        if value.get(key).is_none() {
            errors.push(format!("{key} is required"));
        }
    }
    if !is_sha(
        value
            .get("canonical_text_digest")
            .and_then(Value::as_str)
            .unwrap_or(""),
    ) {
        errors.push("canonical_text_digest must be sha256".to_string());
    }
    if !is_sha(
        value
            .get("receipt_digest")
            .and_then(Value::as_str)
            .unwrap_or(""),
    ) {
        errors.push("receipt_digest must be sha256".to_string());
    }
    semantic_receipt_kind_errors(value, &mut errors);
    errors
}

fn semantic_receipt_kind_errors(value: &Value, errors: &mut Vec<String>) {
    match value
        .get("classifier_implementation_kind")
        .and_then(Value::as_str)
        .unwrap_or("")
    {
        "model" => {
            if value.get("provider_model").is_none() {
                errors.push("model receipts require provider_model".to_string());
            }
            if value.get("prompt_contract_digest").is_none() {
                errors.push("model receipts require prompt_contract_digest".to_string());
            }
        }
        "human_reviewer" if value.get("classifier_evidence").is_none() => {
            errors.push("human reviewer receipts require classifier_evidence".to_string());
        }
        "deterministic_backstop"
            if value.get("provider_model").is_some()
                || value.get("classifier_evidence").is_some() =>
        {
            errors.push("deterministic receipts must not carry external proof".to_string());
        }
        _ => {}
    }
}

fn unique_count(value: &Value, key: &str, expected: usize, errors: &mut Vec<String>) {
    let Some(items) = value.get(key).and_then(Value::as_array) else {
        errors.push(format!("{key} is required"));
        return;
    };
    let set = items.iter().map(Value::to_string).collect::<BTreeSet<_>>();
    if set.len() != items.len() {
        errors.push(format!("{key} contains duplicate values"));
    }
    if items.len() != expected {
        errors.push(format!(
            "{key} expected {expected} values, got {}",
            items.len()
        ));
    }
}

fn is_sha(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].chars().all(|ch| ch.is_ascii_hexdigit())
}
