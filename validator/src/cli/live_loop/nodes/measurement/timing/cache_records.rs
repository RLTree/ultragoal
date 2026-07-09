use serde_json::{Map, Value};
use std::collections::BTreeMap;

use crate::cli::live_loop::nodes::timing::{command_identity, row_authority};
use crate::cli::live_loop::surfaces::surface_by_id;
use std::path::Path;

use super::record_fields::{CACHE_RECORD_FIELDS, TELEMETRY_CACHE_FIELDS};

pub(super) fn replayable_cache_records(
    root: &Path,
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
        if !is_replayable_cache_record(root, row, tier, cache_mode) {
            continue;
        }
        if let Some(key) = cache_record_key(row)
            && let Some(record) = compact_cache_record(row)
        {
            records.insert(key, record);
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

fn is_replayable_cache_record(root: &Path, row: &Value, tier: &str, cache_mode: &str) -> bool {
    let Some(surface) = text(row, "node_id").and_then(surface_by_id) else {
        return false;
    };
    let Some((expected_command, expected_argv)) =
        command_identity::expected(row, surface.id, surface)
    else {
        return false;
    };
    let Some(digests) =
        row_authority::verified_local_digests(root, row, &expected_command, &expected_argv)
    else {
        return false;
    };
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
        && row_authority::proof_kind_is_claim_safe(row, &digests)
        && text(row, "node_id")
            .is_some_and(|node_id| row_authority::rust_test_count_is_claim_safe(row, node_id))
        && row_authority::claim_ceiling_is_source_local(row)
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
        "runtime_execution_model",
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
        && (text(row, "node_id") != Some("live_loop_measurement_rust_tests")
            || row
                .get("verified_local_executed_test_count")
                .and_then(Value::as_u64)
                .is_some_and(|count| count > 0))
}

fn cache_record_key(row: &Value) -> Option<String> {
    Some(format!(
        "{}:{}:{}",
        text(row, "node_id")?,
        text(row, "input_digest")?,
        text(row, "cache_key")?
    ))
}

fn compact_cache_record(row: &Value) -> Option<Value> {
    let object = row.as_object()?;
    let mut record = Map::new();
    for field in CACHE_RECORD_FIELDS {
        if let Some(value) = object.get(*field) {
            record.insert(field.to_string(), value.clone());
        }
    }
    let telemetry = compact_telemetry(object.get("telemetry_reconciliation")?)?;
    record.insert("telemetry_reconciliation".to_string(), telemetry);
    Some(Value::Object(record))
}

fn compact_telemetry(value: &Value) -> Option<Value> {
    let object = value.as_object()?;
    let mut telemetry = Map::new();
    for field in TELEMETRY_CACHE_FIELDS {
        if let Some(value) = object.get(*field) {
            telemetry.insert(field.to_string(), value.clone());
        }
    }
    Some(Value::Object(telemetry))
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}
