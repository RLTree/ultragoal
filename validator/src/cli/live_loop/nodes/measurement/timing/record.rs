use super::super::super::timing::NODE_TIMING_REL;
use super::super::full_command::FullCommandRun;
use super::derived_fields;
use super::failure::{
    measurement_failure_class, measurement_next_repair, measurement_where_failed,
    measurement_why_failed,
};
use super::state::NodeTimingState;
#[cfg(test)]
pub(crate) use super::state::timing_status;
use super::verified_work::VerifiedLocalProof;
use crate::cli::live_loop::LiveLoopCommand;
use crate::cli::live_loop::surfaces::LoopValidationSurface;
use serde_json::{Value, json};
use std::ops::Deref;

#[derive(Clone)]
pub(crate) struct NodeTimingRow {
    value: Value,
    blocks_hot_loop: bool,
}

impl NodeTimingRow {
    pub(crate) fn as_value(&self) -> &Value {
        &self.value
    }

    pub(crate) fn into_value(self) -> Value {
        self.value
    }

    pub(crate) fn blocks_hot_loop(&self) -> bool {
        self.blocks_hot_loop
    }
}

impl Deref for NodeTimingRow {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

pub(crate) fn node_timing_row(
    surface: LoopValidationSurface,
    command: &LiveLoopCommand,
    candidate_digest: &str,
    changed_files_digest: &str,
    audit_context_digest: &str,
    input_digest: &str,
    baseline: &FullCommandRun,
    verified_local: &VerifiedLocalProof,
    affected_set_status: &'static str,
) -> NodeTimingRow {
    let actual_work_duration_ms = verified_local.actual_work.duration_ms;
    let reconciled_command_duration_ms =
        derived_fields::reconciled_command_duration_ms(verified_local);
    let speedup_ratio = baseline.duration_ms / reconciled_command_duration_ms.max(1);
    let output_digest = derived_fields::output_digest(verified_local);
    let result_digest = derived_fields::result_digest(verified_local, &output_digest);
    let failure_class = measurement_failure_class(baseline, verified_local, speedup_ratio);
    let state = NodeTimingState::from_measurement(verified_local, failure_class);
    let blocks_hot_loop = state.validation_status != "pass" || state.speed_claim_status == "failed";
    let (baseline_proof_kind, baseline_invalidation_proof) =
        derived_fields::baseline_reuse_fields(surface, verified_local.proof_kind);
    let mut row = json!({
        "node_id": surface.id,
        "surface": surface.surface,
        "candidate_digest": candidate_digest,
        "tier": command.tier,
        "cache_mode": command.cache_mode,
        "changed_files_digest": changed_files_digest,
        "audit_context_digest": audit_context_digest,
        "input_digest": input_digest,
        "canonical_full_command": surface.canonical_full_command,
        "receipt_path": command.receipt.display().to_string(),
        "timing_status": state.timing_status,
        "failure_class": failure_class,
        "where_failed": measurement_where_failed(surface, baseline, failure_class),
        "why_failed": measurement_why_failed(baseline, failure_class),
        "next_repair": measurement_next_repair(surface, baseline, failure_class),
        "baseline_duration_ms": baseline.duration_ms,
        "baseline_proof_kind": baseline_proof_kind,
        "baseline_invalidation_proof": baseline_invalidation_proof,
        "verified_local_duration_ms": actual_work_duration_ms,
        "telemetry_reconciliation_duration_ms": verified_local.telemetry_reconciliation_duration_ms,
        "reconciled_command_duration_ms": reconciled_command_duration_ms,
        "product_latency_ms": reconciled_command_duration_ms,
        "speedup_ratio": speedup_ratio,
        "required_speedup": "20x",
        "baseline_exit_code": baseline.exit_code,
        "baseline_launch_error": baseline.launch_error,
        "baseline_stdout_digest": baseline.stdout_digest,
        "baseline_stderr_digest": baseline.stderr_digest,
        "baseline_failure": baseline.failure.to_value(),
        "claim_name": "source-local live-loop speed claim",
        "product_behavior_observed": surface.narrow_rerun,
        "proof_surface": derived_fields::proof_surface(verified_local),
        "independent_reconciliation_surface": "same-candidate logs, metrics, traces, explain output, and live-loop timing receipt",
        "claim_ceiling": "source-local loop timing only; readiness release completion final-packet and update_goal remain blocked",
        "affected_set_status": affected_set_status,
        "cache_honesty": "pass",
        "timing_source": NODE_TIMING_REL,
        "claim_impact": "supports_live_loop_node_timing_only_no_readiness_release_completion_update_goal"
    });
    let object = row
        .as_object_mut()
        .expect("live-loop timing row is always an object");
    object.insert(
        "validation_status".to_string(),
        json!(state.validation_status),
    );
    object.insert(
        "validation_cache_status".to_string(),
        json!(state.validation_cache_status),
    );
    object.insert(
        "observability_status".to_string(),
        json!(state.observability_status),
    );
    object.insert(
        "speed_claim_status".to_string(),
        json!(state.speed_claim_status),
    );
    object.insert(
        "observability_failure_class".to_string(),
        json!(state.observability_failure_class),
    );
    object.insert("claim_status".to_string(), json!(state.claim_status));
    super::execution_fields::insert(
        object,
        surface,
        input_digest,
        command.receipt.display().to_string(),
        verified_local,
    );
    object.insert("output_digest".to_string(), json!(output_digest));
    object.insert(
        "verified_local_output_digest".to_string(),
        json!(output_digest),
    );
    object.insert("result_digest".to_string(), json!(result_digest));
    object.insert(
        "verified_local_result_digest".to_string(),
        json!(result_digest),
    );
    NodeTimingRow {
        value: row,
        blocks_hot_loop,
    }
}
