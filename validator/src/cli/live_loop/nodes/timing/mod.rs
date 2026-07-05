use super::super::graph;
use super::super::surfaces::surface_by_id;
use super::command_failure::CommandFailureSummary;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) const NODE_TIMING_REL: &str =
    "validation_artifacts/observability/live-loop-node-timing.json";

#[derive(Clone, Debug)]
pub(crate) struct NodeTiming {
    pub(crate) baseline_duration_ms: u64,
    pub(crate) verified_local_duration_ms: u64,
    pub(crate) proof_kind: String,
    pub(crate) cache_hit: bool,
    pub(crate) cache_key: String,
    pub(crate) work_unit_count: u64,
    pub(crate) actual_work_duration_ms: u64,
    pub(crate) graph_overhead_ms: u64,
    pub(crate) equivalence_status: String,
    pub(crate) invalidation_proof: String,
    pub(crate) telemetry_reconciliation_status: String,
    pub(crate) verified_local_command: String,
    pub(crate) verified_local_result_digest: String,
    pub(crate) verified_local_output_digest: String,
    pub(crate) where_failed: String,
    pub(crate) why_failed: String,
    pub(crate) next_repair: String,
    pub(crate) timing_status: String,
    pub(crate) failure_class: String,
    pub(crate) baseline_exit_code: Option<i32>,
    pub(crate) baseline_launch_error: bool,
    pub(crate) baseline_failure: CommandFailureSummary,
    pub(crate) affected_set_status: String,
    pub(crate) timing_source: String,
}

pub(crate) fn read_current(
    root: &Path,
    candidate_digest: &str,
    tier: &str,
    cache_mode: &str,
    changed_files_digest: &str,
    audit_context_digest: &str,
) -> BTreeMap<String, NodeTiming> {
    let Ok(value) = crate::json_boundary::read_json(&root.join(NODE_TIMING_REL)) else {
        return BTreeMap::new();
    };
    node_rows(&value)
        .into_iter()
        .filter_map(|row| {
            let node_id = text(row, "node_id")?;
            let surface = surface_by_id(node_id)?;
            let expected_input = graph::surface_input_digest(
                surface,
                candidate_digest,
                changed_files_digest,
                audit_context_digest,
            );
            if text(row, "candidate_digest")? != candidate_digest
                || text(row, "tier")? != tier
                || text(row, "cache_mode")? != cache_mode
                || text(row, "input_digest")? != expected_input
                || text(row, "current_input_digest")? != expected_input
                || text(row, "canonical_full_command")? != surface.canonical_full_command
                || text(row, "cache_honesty")? != "pass"
            {
                return None;
            }
            let baseline_duration_ms = positive(row, "baseline_duration_ms")?;
            let verified_local_duration_ms = positive(row, "verified_local_duration_ms")?;
            let proof_kind = text(row, "proof_kind")?;
            let cache_hit = row.get("cache_hit")?.as_bool()?;
            let cache_key = valid_digest(text(row, "cache_key")?)?;
            let work_unit_count = row.get("work_unit_count")?.as_u64()?;
            let actual_work_duration_ms = positive(row, "actual_work_duration_ms")?;
            let graph_overhead_ms = positive(row, "graph_overhead_ms")?;
            if actual_work_duration_ms != verified_local_duration_ms {
                return None;
            }
            let equivalence_status = text(row, "equivalence_status")?;
            let invalidation_proof = text(row, "invalidation_proof")?;
            let telemetry_reconciliation_status = text(row, "telemetry_reconciliation_status")?;
            let validator_version = text(row, "validator_version")?;
            let law_version = text(row, "law_version")?;
            let schema_version = text(row, "schema_version")?;
            let fixture_version = text(row, "fixture_version")?;
            if validator_version.is_empty()
                || law_version.is_empty()
                || schema_version.is_empty()
                || fixture_version.is_empty()
            {
                return None;
            }
            let verified_local_command = text(row, "verified_local_command")?;
            if verified_local_command.is_empty()
                || !has_nonempty_string_array(row, "verified_local_command_argv")
            {
                return None;
            }
            let where_failed = nonempty_text(row, "where_failed")?;
            let why_failed = nonempty_text(row, "why_failed")?;
            let next_repair = nonempty_text(row, "next_repair")?;
            let verified_local_result_digest =
                valid_digest(text(row, "verified_local_result_digest")?)?;
            let verified_local_output_digest =
                valid_digest(text(row, "verified_local_output_digest")?)?;
            valid_digest(text(row, "verified_local_stdout_digest")?)?;
            valid_digest(text(row, "verified_local_stderr_digest")?)?;
            match proof_kind {
                "executed" => {
                    if cache_hit || work_unit_count == 0 {
                        return None;
                    }
                }
                "verified_cache_hit" => {
                    if !cache_hit
                        || text(row, "prior_result_digest")
                            .and_then(valid_digest)
                            .is_none()
                        || text(row, "replayed_output_digest")
                            .and_then(valid_digest)
                            .is_none()
                        || text(row, "cache_equivalence_status") != Some("pass")
                    {
                        return None;
                    }
                }
                _ => return None,
            }
            let baseline_exit_code = row
                .get("baseline_exit_code")
                .and_then(serde_json::Value::as_i64)
                .and_then(|value| i32::try_from(value).ok());
            let baseline_launch_error = row
                .get("baseline_launch_error")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            if !baseline_launch_error && baseline_exit_code.is_none() {
                return None;
            }
            Some((
                node_id.to_string(),
                NodeTiming {
                    baseline_duration_ms,
                    verified_local_duration_ms,
                    proof_kind: proof_kind.to_string(),
                    cache_hit,
                    cache_key: cache_key.to_string(),
                    work_unit_count,
                    actual_work_duration_ms,
                    graph_overhead_ms,
                    equivalence_status: equivalence_status.to_string(),
                    invalidation_proof: invalidation_proof.to_string(),
                    telemetry_reconciliation_status: telemetry_reconciliation_status.to_string(),
                    verified_local_command: verified_local_command.to_string(),
                    verified_local_result_digest: verified_local_result_digest.to_string(),
                    verified_local_output_digest: verified_local_output_digest.to_string(),
                    where_failed: where_failed.to_string(),
                    why_failed: why_failed.to_string(),
                    next_repair: next_repair.to_string(),
                    timing_status: text(row, "timing_status").unwrap_or("fail").to_string(),
                    failure_class: text(row, "failure_class")
                        .unwrap_or("live_loop_node_measurement_failed")
                        .to_string(),
                    baseline_exit_code,
                    baseline_launch_error,
                    baseline_failure: CommandFailureSummary::from_value(
                        row.get("baseline_failure"),
                    ),
                    affected_set_status: text(row, "affected_set_status")
                        .unwrap_or("unknown")
                        .to_string(),
                    timing_source: NODE_TIMING_REL.to_string(),
                },
            ))
        })
        .collect()
}

pub(super) fn node_rows(value: &Value) -> Vec<&Value> {
    value
        .get("nodes")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().collect())
        .unwrap_or_default()
}

pub(super) fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn nonempty_text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    let text = text(value, key)?;
    (!text.is_empty()).then_some(text)
}

pub(super) fn positive(value: &Value, key: &str) -> Option<u64> {
    let number = value.get(key).and_then(Value::as_u64)?;
    (number > 0).then_some(number)
}

fn valid_digest(value: &str) -> Option<&str> {
    value.starts_with("sha256:").then_some(value)
}

fn has_nonempty_string_array(value: &Value, key: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| !items.is_empty() && items.iter().all(|item| item.as_str().is_some()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod timing_record_acceptance;
