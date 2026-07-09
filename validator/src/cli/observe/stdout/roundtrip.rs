use serde_json::Value;

pub(super) fn print(value: &Value) {
    for line in lines(value) {
        println!("{line}");
    }
}

pub(super) fn lines(value: &Value) -> Vec<String> {
    value
        .get("results")
        .and_then(Value::as_array)
        .into_iter()
        .flat_map(|rows| rows.iter())
        .map(|row| line(value, row))
        .collect()
}

fn line(value: &Value, row: &Value) -> String {
    format!(
        "command_roundtrip_result command_id={} product_behavior='{}' proof_surface='{}' independent_reconciliation_surface='{}' candidate={} run_id={} correlation_id={} command_receipt={} artifact_path={} failure_class={} why_failed='{}' where_failed={} next_repair='{}' claim_status={} claim_ceiling='{}'",
        text(row, "command_id", "unknown"),
        text(row, "product_behavior_observed", "unknown"),
        text(row, "proof_surface", "unknown"),
        text(row, "independent_reconciliation_surface", "unknown"),
        text(row, "candidate_digest", "<missing>"),
        text(row, "run_id", "<missing>"),
        text(row, "correlation_id", "<missing>"),
        text(row, "receipt_path", "<missing>"),
        text(row, "artifact_path", "none"),
        text(row, "failure_class", "none"),
        text(row, "why_failed", "none"),
        text(row, "where_failed", "none"),
        text(row, "next_repair", "none"),
        text(row, "claim_status", "unknown"),
        text(value, "claim_ceiling", "unknown")
    )
}

fn text<'a>(value: &'a Value, field: &str, default: &'a str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or(default)
}
