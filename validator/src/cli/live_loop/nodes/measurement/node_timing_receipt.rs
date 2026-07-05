use super::super::super::LiveLoopCommand;
use super::super::super::surfaces::LoopValidationSurface;
use super::super::timing::NODE_TIMING_REL;
use super::full_command::FullCommandRun;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn node_timing_row(
    surface: LoopValidationSurface,
    command: &LiveLoopCommand,
    candidate_digest: &str,
    changed_files_digest: &str,
    audit_context_digest: &str,
    input_digest: &str,
    baseline: &FullCommandRun,
    verified_local_duration_ms: u64,
    affected_set_status: &'static str,
) -> Value {
    let speedup_ratio = baseline.duration_ms / verified_local_duration_ms.max(1);
    let pass = baseline.status_success && speedup_ratio >= 20;
    json!({
        "node_id": surface.id,
        "surface": surface.surface,
        "candidate_digest": candidate_digest,
        "tier": command.tier,
        "cache_mode": command.cache_mode,
        "changed_files_digest": changed_files_digest,
        "audit_context_digest": audit_context_digest,
        "input_digest": input_digest,
        "canonical_full_command": surface.canonical_full_command,
        "timing_status": timing_status(pass),
        "failure_class": measurement_failure_class(baseline, speedup_ratio),
        "baseline_duration_ms": baseline.duration_ms,
        "verified_local_duration_ms": verified_local_duration_ms,
        "speedup_ratio": speedup_ratio,
        "required_speedup": "20x",
        "baseline_exit_code": baseline.exit_code,
        "baseline_launch_error": baseline.launch_error,
        "baseline_stdout_digest": baseline.stdout_digest,
        "baseline_stderr_digest": baseline.stderr_digest,
        "baseline_failure": baseline.failure.to_value(),
        "affected_set_status": affected_set_status,
        "cache_honesty": "pass",
        "timing_source": NODE_TIMING_REL,
        "claim_impact": "supports_live_loop_node_timing_only_no_readiness_release_completion_update_goal"
    })
}

pub(super) fn write_node_timings(
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

pub(super) fn print_measurements(command: &LiveLoopCommand, candidate: &str, rows: &[Value]) {
    for row in rows {
        println!(
            "ultragoal-loop-measure {} candidate={} node={} baseline_duration_ms={} verified_local_duration_ms={} speedup_ratio={} where_failed='{}' why_failed='{}' next_repair='{}' receipt={} claim_ceiling='source-local loop timing only'",
            text(row, "timing_status").unwrap_or("fail"),
            candidate,
            text(row, "node_id").unwrap_or("unknown"),
            positive(row, "baseline_duration_ms").unwrap_or(0),
            positive(row, "verified_local_duration_ms").unwrap_or(0),
            positive(row, "speedup_ratio").unwrap_or(0),
            nested_text(row, "baseline_failure", "where_failed").unwrap_or("none"),
            nested_text(row, "baseline_failure", "why_failed").unwrap_or("none"),
            nested_text(row, "baseline_failure", "next_repair").unwrap_or("none"),
            command.receipt.display()
        );
    }
}

pub(super) fn affected_set_status(changed_files: &[String]) -> &'static str {
    if changed_files.is_empty() {
        "clean_worktree_no_affected_files"
    } else {
        "changed_files_digest_bound"
    }
}

pub(super) fn measurement_failure_class(
    baseline: &FullCommandRun,
    speedup_ratio: u64,
) -> &'static str {
    if baseline.launch_error {
        "canonical_full_command_launch_failed"
    } else if !baseline.status_success {
        "canonical_full_command_failed"
    } else if speedup_ratio < 20 {
        "live_loop_speedup_target_missed"
    } else {
        "none"
    }
}

pub(super) fn timing_status(pass: bool) -> &'static str {
    if pass { "pass" } else { "fail" }
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

fn nested_text<'a>(value: &'a Value, object: &str, key: &str) -> Option<&'a str> {
    value.get(object)?.get(key).and_then(Value::as_str)
}
