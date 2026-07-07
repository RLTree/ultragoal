use serde_json::{Map, Value};
use std::collections::BTreeMap;

pub(super) fn replayable_cache_records(
    existing: &Value,
    latest_nodes: &[Value],
    tier: &str,
    cache_mode: &str,
) -> Vec<Value> {
    let mut records = BTreeMap::new();
    for row in cache_row_sources(existing)
        .into_iter()
        .chain(latest_nodes.iter())
    {
        if !is_replayable_cache_record(row, tier, cache_mode) {
            continue;
        }
        if let Some(key) = cache_record_key(row) {
            records.insert(key, row.clone());
        }
    }
    records.into_values().collect()
}

fn cache_row_sources(value: &Value) -> Vec<&Value> {
    let mut rows = Vec::new();
    if let Some(cache_records) = value.get("cache_records").and_then(Value::as_array) {
        rows.extend(cache_records.iter());
    }
    if let Some(nodes) = value.get("nodes").and_then(Value::as_array) {
        rows.extend(nodes.iter());
    }
    rows
}

fn is_replayable_cache_record(row: &Value, tier: &str, cache_mode: &str) -> bool {
    text(row, "tier") == Some(tier)
        && text(row, "cache_mode") == Some(cache_mode)
        && text(row, "cache_honesty") == Some("pass")
        && text(row, "validation_status") == Some("pass")
        && text(row, "validation_cache_status") == Some("reusable")
        && row.get("exit_status").and_then(Value::as_i64) == Some(0)
        && row
            .get("verified_local_launch_error")
            .and_then(Value::as_bool)
            == Some(false)
        && matches!(
            text(row, "proof_kind"),
            Some("executed" | "verified_cache_hit")
        )
        && row
            .get("telemetry_reconciliation")
            .and_then(Value::as_object)
            .is_some_and(telemetry_status_present)
        && required_cache_fields_present(row)
}

fn telemetry_status_present(object: &Map<String, Value>) -> bool {
    object
        .get("status")
        .and_then(Value::as_str)
        .is_some_and(|status| !status.is_empty())
}

fn required_cache_fields_present(row: &Value) -> bool {
    [
        "node_id",
        "input_digest",
        "current_input_digest",
        "canonical_full_command",
        "cache_key",
        "validator_version",
        "law_version",
        "schema_version",
        "fixture_version",
        "validation_status",
        "validation_cache_status",
        "observability_status",
        "speed_claim_status",
        "result_digest",
        "output_digest",
        "verified_local_result_digest",
        "verified_local_output_digest",
        "verified_local_stdout_digest",
        "verified_local_stderr_digest",
        "baseline_stdout_digest",
        "baseline_stderr_digest",
    ]
    .into_iter()
    .all(|field| text(row, field).is_some_and(|value| !value.is_empty()))
}

fn cache_record_key(row: &Value) -> Option<String> {
    Some(format!(
        "{}:{}:{}",
        text(row, "node_id")?,
        text(row, "input_digest")?,
        text(row, "cache_key")?
    ))
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}
