use super::record::NodeTimingRow;
use crate::cli::live_loop::LiveLoopCommand;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(crate) fn write_node_timings(
    root: &Path,
    receipt: &Path,
    candidate: &str,
    tier: &str,
    cache_mode: &str,
    rows: Vec<NodeTimingRow>,
) -> Result<(), String> {
    let path = crate::output_path::claim_artifact_path(root, receipt, "live loop node timing")?;
    let existing = crate::json_boundary::read_json(&path).unwrap_or_else(|_| json!({"nodes":[]}));
    let measured_ids: BTreeSet<String> = rows
        .iter()
        .filter_map(|row| text(row.as_value(), "node_id").map(str::to_string))
        .collect();
    let mut nodes: Vec<Value> = node_rows(&existing)
        .into_iter()
        .filter(|existing_row| {
            text(existing_row, "candidate_digest") == Some(candidate)
                && text(existing_row, "tier") == Some(tier)
                && text(existing_row, "cache_mode") == Some(cache_mode)
                && text(existing_row, "node_id")
                    .map(|node_id| !measured_ids.contains(node_id))
                    .unwrap_or(false)
        })
        .cloned()
        .collect();
    nodes.extend(rows.into_iter().map(NodeTimingRow::into_value));
    nodes.sort_by(|left, right| text(left, "node_id").cmp(&text(right, "node_id")));
    let cache_records = replayable_cache_records(&existing, &nodes, tier, cache_mode);
    crate::json_boundary::write_json(
        &path,
        &json!({
            "schema": "harness-ultragoal.live-loop-node-timing.v1",
            "candidate_digest": candidate,
            "tier": tier,
            "cache_mode": cache_mode,
            "nodes": nodes,
            "cache_records": cache_records
        }),
    )
}

pub(crate) fn print_measurements(
    command: &LiveLoopCommand,
    candidate: &str,
    rows: &[NodeTimingRow],
) {
    for row in rows {
        let row = row.as_value();
        println!(
            "ultragoal-loop-measure {} candidate={} node={} validation_status={} validation_cache_status={} observability_status={} speed_claim_status={} proof_kind={} cache_hit={} work_unit_count={} actual_work_duration_ms={} graph_overhead_ms={} telemetry_reconciliation_duration_ms={} reconciled_command_duration_ms={} telemetry_reconciliation={} baseline_duration_ms={} verified_local_duration_ms={} speedup_ratio={} failure_class={} observability_failure_class={} where_failed='{}' why_failed='{}' next_repair='{}' receipt={} claim_ceiling='source-local loop timing only'",
            text(row, "timing_status").unwrap_or("fail"),
            candidate,
            text(row, "node_id").unwrap_or("unknown"),
            text(row, "validation_status").unwrap_or("unknown"),
            text(row, "validation_cache_status").unwrap_or("unknown"),
            text(row, "observability_status").unwrap_or("unknown"),
            text(row, "speed_claim_status").unwrap_or("withheld"),
            text(row, "proof_kind").unwrap_or("missing"),
            row.get("cache_hit")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            positive(row, "work_unit_count").unwrap_or(0),
            positive(row, "actual_work_duration_ms").unwrap_or(0),
            positive(row, "graph_overhead_ms").unwrap_or(0),
            positive(row, "telemetry_reconciliation_duration_ms").unwrap_or(0),
            positive(row, "reconciled_command_duration_ms").unwrap_or(0),
            text(row, "telemetry_reconciliation_status").unwrap_or("missing"),
            positive(row, "baseline_duration_ms").unwrap_or(0),
            positive(row, "verified_local_duration_ms").unwrap_or(0),
            positive(row, "speedup_ratio").unwrap_or(0),
            text(row, "failure_class").unwrap_or("live_loop_node_measurement_failed"),
            text(row, "observability_failure_class").unwrap_or("none"),
            text(row, "where_failed").unwrap_or("none"),
            text(row, "why_failed").unwrap_or("none"),
            text(row, "next_repair").unwrap_or("none"),
            command.receipt.display()
        );
    }
}

pub(crate) fn affected_set_status(changed_file_count: usize) -> &'static str {
    if changed_file_count == 0 {
        "clean_worktree_no_affected_files"
    } else {
        "changed_files_digest_bound"
    }
}

fn node_rows(value: &Value) -> Vec<&Value> {
    value
        .get("nodes")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().collect())
        .unwrap_or_default()
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn positive(value: &Value, key: &str) -> Option<u64> {
    let number = value.get(key).and_then(Value::as_u64)?;
    (number > 0).then_some(number)
}

fn replayable_cache_records(
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

#[cfg(test)]
#[path = "../cache/record_tests.rs"]
mod tests;
