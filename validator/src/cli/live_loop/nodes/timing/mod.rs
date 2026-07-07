use super::super::graph;
use super::super::surfaces::surface_by_id;
use super::command_failure::CommandFailureSummary;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

mod record_fields;

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
    pub(crate) reconciled_command_duration_ms: u64,
    pub(crate) product_latency_ms: u64,
    pub(crate) equivalence_status: String,
    pub(crate) invalidation_proof: String,
    pub(crate) telemetry_reconciliation_status: String,
    pub(crate) verified_local_command: String,
    pub(crate) result_digest: String,
    pub(crate) output_digest: String,
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
    record_fields::node_rows(&value)
        .into_iter()
        .filter_map(|row| {
            let node_id = record_fields::text(row, "node_id")?;
            let surface = surface_by_id(node_id)?;
            let expected_input = graph::surface_input_digest(
                surface,
                candidate_digest,
                changed_files_digest,
                audit_context_digest,
            );
            if record_fields::text(row, "candidate_digest")? != candidate_digest
                || record_fields::text(row, "tier")? != tier
                || record_fields::text(row, "cache_mode")? != cache_mode
                || record_fields::text(row, "input_digest")? != expected_input
                || record_fields::text(row, "current_input_digest")? != expected_input
                || record_fields::text(row, "canonical_full_command")?
                    != surface.canonical_full_command
                || record_fields::text(row, "cache_honesty")? != "pass"
            {
                return None;
            }
            let baseline_duration_ms = record_fields::positive(row, "baseline_duration_ms")?;
            let verified_local_duration_ms =
                record_fields::positive(row, "verified_local_duration_ms")?;
            let proof_kind = record_fields::text(row, "proof_kind")?;
            let cache_hit = row.get("cache_hit")?.as_bool()?;
            let cache_key = record_fields::valid_digest(record_fields::text(row, "cache_key")?)?;
            let work_unit_count = row.get("work_unit_count")?.as_u64()?;
            let actual_work_duration_ms = record_fields::positive(row, "actual_work_duration_ms")?;
            let graph_overhead_ms = record_fields::positive(row, "graph_overhead_ms")?;
            let reconciled_command_duration_ms =
                record_fields::positive(row, "reconciled_command_duration_ms")?;
            let product_latency_ms = record_fields::positive(row, "product_latency_ms")?;
            if actual_work_duration_ms != verified_local_duration_ms {
                return None;
            }
            let expected_reconciled_ms = actual_work_duration_ms
                .saturating_add(graph_overhead_ms)
                .saturating_add(record_fields::positive(
                    row,
                    "telemetry_reconciliation_duration_ms",
                )?);
            if reconciled_command_duration_ms != expected_reconciled_ms
                || product_latency_ms != reconciled_command_duration_ms
            {
                return None;
            }
            let equivalence_status = record_fields::text(row, "equivalence_status")?;
            let invalidation_proof = record_fields::text(row, "invalidation_proof")?;
            let telemetry_reconciliation_status =
                record_fields::text(row, "telemetry_reconciliation_status")?;
            let version_fields = [
                record_fields::text(row, "validator_version")?,
                record_fields::text(row, "law_version")?,
                record_fields::text(row, "schema_version")?,
                record_fields::text(row, "fixture_version")?,
            ];
            if version_fields.iter().any(|value| value.is_empty()) {
                return None;
            }
            let verified_local_command = record_fields::text(row, "verified_local_command")?;
            if verified_local_command.is_empty()
                || !record_fields::has_nonempty_string_array(row, "command_argv")
            {
                return None;
            }
            row.get("exit_status").and_then(Value::as_i64)?;
            if !record_fields::has_nonempty_string_array(row, "receipt_paths")
                && !record_fields::has_nonempty_string_array(row, "artifact_paths")
            {
                return None;
            }
            let where_failed = record_fields::nonempty_text(row, "where_failed")?;
            let why_failed = record_fields::nonempty_text(row, "why_failed")?;
            let next_repair = record_fields::nonempty_text(row, "next_repair")?;
            let verified_local_result_digest = record_fields::valid_digest(record_fields::text(
                row,
                "verified_local_result_digest",
            )?)?;
            let verified_local_output_digest = record_fields::valid_digest(record_fields::text(
                row,
                "verified_local_output_digest",
            )?)?;
            let result_digest =
                record_fields::valid_digest(record_fields::text(row, "result_digest")?)?;
            let output_digest =
                record_fields::valid_digest(record_fields::text(row, "output_digest")?)?;
            if result_digest != verified_local_result_digest
                || output_digest != verified_local_output_digest
            {
                return None;
            }
            record_fields::valid_digest(record_fields::text(row, "verified_local_stdout_digest")?)?;
            record_fields::valid_digest(record_fields::text(row, "verified_local_stderr_digest")?)?;
            let timing_status = record_fields::text(row, "timing_status").unwrap_or("fail");
            let failure_class = record_fields::text(row, "failure_class")
                .unwrap_or("live_loop_node_measurement_failed");
            if timing_status == "pass"
                && (failure_class != "none" || telemetry_reconciliation_status != "pass")
            {
                return None;
            }
            match proof_kind {
                "executed" => {
                    if cache_hit
                        || work_unit_count == 0
                        || equivalence_status != "executed_current_candidate_not_cache_replay"
                        || invalidation_proof.is_empty()
                    {
                        return None;
                    }
                }
                "verified_cache_hit" => {
                    let prior_result_digest = record_fields::text(row, "prior_result_digest")
                        .and_then(record_fields::valid_digest)?;
                    let replayed_output_digest = record_fields::text(row, "replayed_output_digest")
                        .and_then(record_fields::valid_digest)?;
                    if !cache_hit
                        || work_unit_count != 0
                        || prior_result_digest != result_digest
                        || replayed_output_digest != output_digest
                        || record_fields::text(row, "cache_equivalence_status") != Some("pass")
                        || equivalence_status != "verified_same_candidate_cache_replay"
                        || invalidation_proof.is_empty()
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
                    reconciled_command_duration_ms,
                    product_latency_ms,
                    equivalence_status: equivalence_status.to_string(),
                    invalidation_proof: invalidation_proof.to_string(),
                    telemetry_reconciliation_status: telemetry_reconciliation_status.to_string(),
                    verified_local_command: verified_local_command.to_string(),
                    result_digest: result_digest.to_string(),
                    output_digest: output_digest.to_string(),
                    verified_local_result_digest: verified_local_result_digest.to_string(),
                    verified_local_output_digest: verified_local_output_digest.to_string(),
                    where_failed: where_failed.to_string(),
                    why_failed: why_failed.to_string(),
                    next_repair: next_repair.to_string(),
                    timing_status: timing_status.to_string(),
                    failure_class: failure_class.to_string(),
                    baseline_exit_code,
                    baseline_launch_error,
                    baseline_failure: CommandFailureSummary::from_value(
                        row.get("baseline_failure"),
                    ),
                    affected_set_status: record_fields::text(row, "affected_set_status")
                        .unwrap_or("unknown")
                        .to_string(),
                    timing_source: NODE_TIMING_REL.to_string(),
                },
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod timing_record_acceptance;
