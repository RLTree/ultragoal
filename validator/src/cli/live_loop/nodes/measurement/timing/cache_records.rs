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

const CACHE_RECORD_FIELDS: &[&str] = &[
    "node_id",
    "surface",
    "candidate_digest",
    "tier",
    "cache_mode",
    "changed_files_digest",
    "audit_context_digest",
    "input_digest",
    "current_input_digest",
    "canonical_full_command",
    "receipt_path",
    "timing_status",
    "failure_class",
    "where_failed",
    "why_failed",
    "next_repair",
    "baseline_duration_ms",
    "baseline_proof_kind",
    "baseline_invalidation_proof",
    "verified_local_duration_ms",
    "telemetry_reconciliation_duration_ms",
    "reconciled_command_duration_ms",
    "product_latency_ms",
    "speedup_ratio",
    "required_speedup",
    "baseline_exit_code",
    "baseline_launch_error",
    "baseline_stdout_digest",
    "baseline_stderr_digest",
    "baseline_failure",
    "claim_name",
    "product_behavior_observed",
    "proof_surface",
    "independent_reconciliation_surface",
    "claim_ceiling",
    "affected_set_status",
    "cache_honesty",
    "timing_source",
    "claim_impact",
    "validation_status",
    "validation_cache_status",
    "observability_status",
    "speed_claim_status",
    "observability_failure_class",
    "claim_status",
    "proof_kind",
    "cache_hit",
    "cache_key",
    "graph_overhead_ms",
    "actual_work_duration_ms",
    "work_unit_count",
    "equivalence_status",
    "invalidation_proof",
    "telemetry_reconciliation_status",
    "current_input_digest",
    "validator_version",
    "law_version",
    "schema_version",
    "fixture_version",
    "runtime_execution_model",
    "surface_input_spec_status",
    "surface_input_spec_node_id",
    "surface_input_spec_cache_boundary",
    "validator_authority",
    "environment_class",
    "cache_class",
    "claim_surface",
    "output_digest_expectation",
    "verified_local_command",
    "command_argv",
    "receipt_paths",
    "artifact_paths",
    "queue_depth",
    "worker_count",
    "task_count",
    "execution_class",
    "exit_status",
    "verified_local_launch_error",
    "verified_local_stdout_digest",
    "verified_local_stderr_digest",
    "verified_local_output_digest",
    "verified_local_result_digest",
    "output_digest",
    "result_digest",
    "prior_result_digest",
    "replayed_output_digest",
    "cache_equivalence_status",
];

const TELEMETRY_CACHE_FIELDS: &[&str] = &[
    "status",
    "run_id",
    "correlation_id",
    "trace_id",
    "span_id",
    "command_observation_receipt",
    "logs_query",
    "metrics_query",
    "traces_query",
    "explain_failure",
    "cached_reconciliation",
    "first_failed_roundtrip",
    "observability_failure_class",
    "failure_class",
    "where_failed",
    "why_failed",
    "next_repair",
    "claim_impact",
];

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}
