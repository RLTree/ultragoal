use super::NODE_TIMING_REL;
use crate::cli::live_loop;
use serde_json::{Map, Value, json};
use std::path::Path;

pub(super) fn current_nodes(root: &Path, candidate: &str) -> Vec<Value> {
    let timing = crate::json_boundary::read_json(&root.join(NODE_TIMING_REL))
        .unwrap_or_else(|_| json!({"nodes":[]}));
    let mut nodes = timing
        .get("nodes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|row| text(row, "candidate_digest") == Some(candidate))
        .filter(|row| row_matches_current_live_loop_context(root, candidate, row))
        .filter_map(project_node)
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| text(left, "node_id").cmp(&text(right, "node_id")));
    nodes
}

fn project_node(row: &Value) -> Option<Value> {
    text(row, "node_id")?;
    let mut object = Map::new();
    for key in [
        "node_id",
        "proof_kind",
        "candidate_digest",
        "tier",
        "cache_mode",
        "changed_files_digest",
        "audit_context_digest",
        "input_digest",
        "current_input_digest",
        "cache_key",
        "validator_version",
        "law_version",
        "schema_version",
        "fixture_version",
        "result_digest",
        "output_digest",
        "telemetry_reconciliation_status",
        "validation_status",
        "validation_cache_status",
        "observability_status",
        "speed_claim_status",
        "observability_failure_class",
        "claim_impact",
        "timing_status",
        "failure_class",
        "where_failed",
        "why_failed",
        "next_repair",
        "required_speedup",
        "surface",
        "claim_name",
        "product_behavior_observed",
        "proof_surface",
        "independent_reconciliation_surface",
    ] {
        insert_text(row, &mut object, key);
    }
    for key in [
        "cache_hit",
        "work_unit_count",
        "actual_work_duration_ms",
        "graph_overhead_ms",
        "telemetry_reconciliation_duration_ms",
        "reconciled_command_duration_ms",
        "product_latency_ms",
        "speedup_ratio",
        "baseline_duration_ms",
        "verified_local_duration_ms",
    ] {
        insert_value(row, &mut object, key);
    }
    insert_command_fields(row, &mut object);
    insert_cache_replay_fields(row, &mut object);
    Some(Value::Object(object))
}

fn row_matches_current_live_loop_context(root: &Path, candidate: &str, row: &Value) -> bool {
    let Some(node_id) = text(row, "node_id") else {
        return false;
    };
    let Some(tier) = text(row, "tier") else {
        return false;
    };
    let Some(cache_mode) = text(row, "cache_mode") else {
        return false;
    };
    let Some(context) =
        live_loop::validation_surface_context(root, candidate, node_id, tier, cache_mode)
    else {
        return false;
    };
    text(row, "input_digest") == Some(context.input_digest.as_str())
        && text(row, "current_input_digest") == Some(context.input_digest.as_str())
        && text(row, "audit_context_digest") == Some(context.audit_context_digest.as_str())
        && text(row, "cache_key") == Some(context.cache_key.as_str())
        && text(row, "validator_version") == Some(context.validator_version.as_str())
        && text(row, "law_version") == Some(context.law_version)
        && text(row, "schema_version") == Some(context.schema_version)
        && text(row, "fixture_version") == Some(context.fixture_version)
}

fn insert_command_fields(row: &Value, object: &mut Map<String, Value>) {
    if let Some(argv) = string_array(row, "command_argv") {
        object.insert("command_argv".to_string(), json!(argv));
    }
    if let Some(code) = row.get("exit_status").and_then(Value::as_i64) {
        object.insert("exit_status".to_string(), json!(code));
    }
    if let Some(paths) = string_array(row, "receipt_paths") {
        object.insert("receipt_paths".to_string(), json!(paths));
    }
    if let Some(paths) = string_array(row, "artifact_paths") {
        object.insert("artifact_paths".to_string(), json!(paths));
    }
}

fn insert_cache_replay_fields(row: &Value, object: &mut Map<String, Value>) {
    if text(row, "proof_kind") != Some("verified_cache_hit") {
        return;
    }
    for key in [
        "cache_key",
        "current_input_digest",
        "validator_version",
        "law_version",
        "schema_version",
        "fixture_version",
        "prior_result_digest",
        "replayed_output_digest",
        "equivalence_status",
        "invalidation_proof",
    ] {
        insert_text(row, object, key);
    }
}

pub(super) fn string_array(row: &Value, key: &str) -> Option<Vec<String>> {
    row.get(key)?
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(str::to_string))
        .collect()
}

fn insert_text(row: &Value, object: &mut Map<String, Value>, key: &str) {
    if let Some(value) = text(row, key) {
        object.insert(key.to_string(), json!(value));
    }
}

fn insert_value(row: &Value, object: &mut Map<String, Value>, key: &str) {
    if let Some(value) = row.get(key) {
        object.insert(key.to_string(), value.clone());
    }
}

pub(super) fn text<'a>(row: &'a Value, key: &str) -> Option<&'a str> {
    row.get(key).and_then(Value::as_str)
}
