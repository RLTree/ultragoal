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
    rows: Vec<Value>,
) -> Result<(), String> {
    let path = crate::output_path::claim_artifact_path(root, receipt, "live loop node timing")?;
    let existing = crate::json_boundary::read_json(&path).unwrap_or_else(|_| json!({"nodes":[]}));
    let measured_ids: BTreeSet<String> = rows
        .iter()
        .filter_map(|row| text(row, "node_id").map(str::to_string))
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
    nodes.extend(rows);
    nodes.sort_by(|left, right| text(left, "node_id").cmp(&text(right, "node_id")));
    crate::json_boundary::write_json(
        &path,
        &json!({
            "schema": "harness-ultragoal.live-loop-node-timing.v1",
            "candidate_digest": candidate,
            "tier": tier,
            "cache_mode": cache_mode,
            "nodes": nodes
        }),
    )
}

pub(crate) fn print_measurements(command: &LiveLoopCommand, candidate: &str, rows: &[Value]) {
    for row in rows {
        println!(
            "ultragoal-loop-measure {} candidate={} node={} proof_kind={} cache_hit={} work_unit_count={} actual_work_duration_ms={} graph_overhead_ms={} telemetry_reconciliation_duration_ms={} reconciled_command_duration_ms={} telemetry_reconciliation={} baseline_duration_ms={} verified_local_duration_ms={} speedup_ratio={} failure_class={} where_failed='{}' why_failed='{}' next_repair='{}' receipt={} claim_ceiling='source-local loop timing only'",
            text(row, "timing_status").unwrap_or("fail"),
            candidate,
            text(row, "node_id").unwrap_or("unknown"),
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
            text(row, "where_failed").unwrap_or("none"),
            text(row, "why_failed").unwrap_or("none"),
            text(row, "next_repair").unwrap_or("none"),
            command.receipt.display()
        );
    }
}

pub(crate) fn affected_set_status(changed_files: &[String]) -> &'static str {
    if changed_files.is_empty() {
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
