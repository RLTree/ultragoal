use super::super::surfaces::surface_by_id;
use super::super::{changed_inputs::ChangedInputs, graph};
use super::command_failure::CommandFailureSummary;
pub(crate) use node_timing::{NodeTiming, TelemetryReconciliationRecord};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

mod node_timing;
mod record_fields;

pub(crate) const NODE_TIMING_REL: &str =
    "validation_artifacts/observability/live-loop-node-timing.json";
pub(crate) const VALIDATION_CACHE_REL: &str =
    "validation_artifacts/observability/live-loop-validation-cache.json";

pub(crate) fn read_current(
    root: &Path,
    candidate_digest: &str,
    tier: &str,
    cache_mode: &str,
    changed_inputs: &ChangedInputs,
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
                changed_inputs.surface_digest(surface),
                &changed_inputs.audit_context_digest,
            );
            let row_candidate = record_fields::text(row, "candidate_digest")?;
            if record_fields::text(row, "tier")? != tier
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
            let expected_cache_key =
                graph::verified_local_cache_key(surface, &expected_input, tier, cache_mode);
            if cache_key != expected_cache_key {
                return None;
            }
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
            let expected_product_latency_ms =
                actual_work_duration_ms.saturating_add(graph_overhead_ms);
            if reconciled_command_duration_ms != expected_reconciled_ms
                || product_latency_ms != expected_product_latency_ms
            {
                return None;
            }
            let equivalence_status = record_fields::text(row, "equivalence_status")?;
            let invalidation_proof = record_fields::text(row, "invalidation_proof")?;
            if row_candidate != candidate_digest && !surface.high_frequency {
                return None;
            }
            let telemetry_reconciliation_status =
                record_fields::text(row, "telemetry_reconciliation_status")?;
            let validation_status = record_fields::nonempty_text(row, "validation_status")?;
            let validation_cache_status =
                record_fields::nonempty_text(row, "validation_cache_status")?;
            let observability_status = record_fields::nonempty_text(row, "observability_status")?;
            let speed_claim_status = record_fields::nonempty_text(row, "speed_claim_status")?;
            let observability_failure_class =
                record_fields::nonempty_text(row, "observability_failure_class")?;
            if !record_fields::runtime_versions_match(row) {
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
            let baseline_proof_kind = record_fields::nonempty_text(row, "baseline_proof_kind")?;
            let baseline_invalidation_proof =
                record_fields::nonempty_text(row, "baseline_invalidation_proof")?;
            let telemetry_reconciliation =
                TelemetryReconciliationRecord::from_value(row.get("telemetry_reconciliation")?)?;
            if timing_status == "pass"
                && (failure_class != "none" || telemetry_reconciliation_status != "pass")
            {
                return None;
            }
            if timing_status == "partial"
                && (validation_status != "pass" || speed_claim_status != "withheld")
            {
                return None;
            }
            if timing_status == "fail" && failure_class == "none" {
                return None;
            }
            match proof_kind {
                "executed" => {
                    if cache_hit
                        || row_candidate != candidate_digest
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
                    validation_status: validation_status.to_string(),
                    validation_cache_status: validation_cache_status.to_string(),
                    observability_status: observability_status.to_string(),
                    speed_claim_status: speed_claim_status.to_string(),
                    observability_failure_class: observability_failure_class.to_string(),
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
                    baseline_proof_kind: baseline_proof_kind.to_string(),
                    baseline_invalidation_proof: baseline_invalidation_proof.to_string(),
                    baseline_exit_code,
                    baseline_launch_error,
                    baseline_failure: CommandFailureSummary::from_value(
                        row.get("baseline_failure"),
                    ),
                    telemetry_reconciliation,
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
mod candidate_boundary_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod timing_record_acceptance;
