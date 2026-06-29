use serde_json::Value;

pub(super) fn failures(value: &Value, expected: &str) -> Vec<String> {
    let mut out = Vec::new();
    if value
        .pointer("/target_revision/value")
        .and_then(Value::as_str)
        != Some(expected)
    {
        out.push("final_packet_proof_coverage_target_digest_mismatch".to_string());
    }
    if value.pointer("/coverage/percent").and_then(Value::as_f64) != Some(100.0)
        || !value
            .get("uncovered_records")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
    {
        out.push("final_packet_proof_coverage_not_exact_100".to_string());
    }
    if value.get("claim_ceiling").and_then(Value::as_str)
        != Some("supports_complete_coverage_claim")
    {
        out.push("final_packet_proof_coverage_claim_ceiling_not_complete".to_string());
    }
    if !array_contains(value, "supported_claim_classes", "complete_coverage") {
        out.push("final_packet_proof_coverage_supported_claim_missing".to_string());
    }
    for blocked in [
        "completion",
        "package_readiness",
        "review_readiness",
        "release_readiness",
        "final_packet_correctness",
        "update_goal_eligibility",
        "app_registry_or_reviewer_exposure",
    ] {
        if !array_contains(value, "blocked_claim_classes", blocked) {
            out.push(format!(
                "final_packet_proof_coverage_missing_blocked_claim:{blocked}"
            ));
        }
    }
    out
}

fn array_contains(value: &Value, key: &str, needle: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(needle)))
}
