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
    if value.get("claim_ceiling").and_then(Value::as_str) != Some("supports_complete_claim") {
        out.push("final_packet_proof_coverage_claim_ceiling_not_complete".to_string());
    }
    out
}
