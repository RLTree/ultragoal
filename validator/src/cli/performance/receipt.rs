use crate::cli::performance::types::PERFORMANCE_RECEIPT_SCHEMA;
use serde_json::Value;

pub(crate) fn surface_value_failures(value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if value.get("schema").and_then(Value::as_str) != Some(PERFORMANCE_RECEIPT_SCHEMA) {
        out.push("cli_performance_receipt_wrong_schema".to_string());
    }
    let status = value.get("status").and_then(Value::as_str);
    for ptr in [
        "/command/argv",
        "/budget/class",
        "/digests/candidate",
        "/cache/mode",
        "/concurrency/worker_count",
        "/telemetry/wall_clock_ms",
    ] {
        if value.pointer(ptr).is_none() {
            out.push(format!("cli_performance_receipt_missing:{ptr}"));
        }
    }
    if status == Some("fail") && value.pointer("/failure/check_id").is_none() {
        out.push("cli_performance_receipt_missing:/failure/check_id".to_string());
    }
    if status == Some("pass")
        && value.get("claim_ceiling").and_then(Value::as_str) != Some("performance_proven")
    {
        out.push("cli_performance_pass_without_positive_claim_ceiling".to_string());
    }
    if status == Some("pass")
        && value
            .get("blocked_claim_classes")
            .and_then(Value::as_array)
            .is_some_and(|rows| !rows.is_empty())
    {
        out.push("cli_performance_pass_with_blocked_claims".to_string());
    }
    let blocked = value
        .get("blocked_claim_classes")
        .and_then(Value::as_array)
        .is_some_and(|rows| !rows.is_empty());
    if status == Some("fail") && !blocked {
        out.push("cli_performance_fail_without_blocked_claims".to_string());
    }
    out
}
