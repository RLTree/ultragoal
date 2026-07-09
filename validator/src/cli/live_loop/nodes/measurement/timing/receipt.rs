use super::super::super::timing::VALIDATION_CACHE_REL;
use super::cache_records::replayable_cache_records;
use super::record::NodeTimingRow;
use crate::cli::live_loop::LiveLoopCommand;
use serde_json::{Value, json};
use std::collections::BTreeSet;
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
    let cache_records = replayable_cache_records(root, &existing, &nodes, tier, cache_mode);
    let cache_path = crate::output_path::literal_claim_artifact_path(
        root,
        VALIDATION_CACHE_REL,
        "live loop validation cache",
    );
    crate::json_boundary::write_json(
        &cache_path,
        &json!({
            "schema": "harness-ultragoal.live-loop-validation-cache.v1",
            "candidate_digest": candidate,
            "tier": tier,
            "cache_mode": cache_mode,
            "records": cache_records.clone()
        }),
    )?;
    crate::json_boundary::write_json(
        &path,
        &json!({
            "schema": crate::cli::live_loop::graph::schema_version(),
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
            "ultragoal-loop-measure {} candidate={} node={} validation_status={} validation_cache_status={} observability_status={} speed_claim_status={} proof_kind={} cache_hit={} work_unit_count={} actual_work_duration_ms={} graph_overhead_ms={} telemetry_reconciliation_duration_ms={} reconciled_command_duration_ms={} telemetry_reconciliation={} baseline_duration_ms={} verified_local_duration_ms={} speedup_ratio={} failure_class={} observability_failure_class={} where_failed='{}' why_failed='{}' next_repair='{}' receipt={} run_id={} correlation_id={} trace_id={} command_observation_receipt={} first_failed_roundtrip='{}' query_logs='{}' query_metrics='{}' query_traces='{}' claim_ceiling='source-local loop timing only'",
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
            command.receipt.display(),
            telemetry_text(row, "run_id").unwrap_or("unknown"),
            telemetry_text(row, "correlation_id").unwrap_or("unknown"),
            telemetry_text(row, "trace_id").unwrap_or("unknown"),
            telemetry_text(row, "command_observation_receipt").unwrap_or("unknown"),
            telemetry_json(row, "first_failed_roundtrip"),
            telemetry_query(row, "logs_query").unwrap_or("unknown"),
            telemetry_query(row, "metrics_query").unwrap_or("unknown"),
            telemetry_query(row, "traces_query").unwrap_or("unknown")
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

fn telemetry_text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    let telemetry = value.get("telemetry_reconciliation")?;
    telemetry
        .get(key)
        .or_else(|| {
            telemetry
                .get("cached_reconciliation")
                .and_then(|cached| cached.get(key))
        })
        .and_then(Value::as_str)
}

fn telemetry_query<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    let telemetry = value.get("telemetry_reconciliation")?;
    telemetry
        .get(key)
        .or_else(|| {
            telemetry
                .get("cached_reconciliation")
                .and_then(|cached| cached.get(key))
        })
        .and_then(query_text)
}

fn query_text(value: &Value) -> Option<&str> {
    value.get("query").and_then(Value::as_str).or_else(|| {
        value
            .get("value")
            .and_then(|query_value| query_value.get("query"))
            .and_then(Value::as_str)
    })
}

fn telemetry_json(value: &Value, key: &str) -> String {
    value
        .get("telemetry_reconciliation")
        .and_then(|telemetry| {
            telemetry.get(key).or_else(|| {
                telemetry
                    .get("cached_reconciliation")
                    .and_then(|cached| cached.get(key))
            })
        })
        .and_then(|item| serde_json::to_string(item).ok())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
#[path = "../../../node_timing/tests/mod.rs"]
mod tests;
