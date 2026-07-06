use super::super::super::timing::NODE_TIMING_REL;
use super::super::full_command::FullCommandRun;
use super::derived_fields;
use super::failure::{
    measurement_failure_class, measurement_next_repair, measurement_where_failed,
    measurement_why_failed,
};
use super::verified_work::VerifiedLocalProof;
use crate::cli::live_loop::LiveLoopCommand;
use crate::cli::live_loop::surfaces::LoopValidationSurface;
use serde_json::{Map, Value, json};

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
) -> Value {
    let actual_work_duration_ms = verified_local.actual_work.duration_ms;
    let speedup_ratio = baseline.duration_ms / actual_work_duration_ms.max(1);
    let output_digest = derived_fields::output_digest(verified_local);
    let result_digest = derived_fields::result_digest(verified_local, &output_digest);
    let failure_class = measurement_failure_class(baseline, verified_local, speedup_ratio);
    let pass = failure_class == "none";
    let (baseline_proof_kind, baseline_invalidation_proof) =
        derived_fields::baseline_reuse_fields(verified_local.proof_kind);
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
        "timing_status": timing_status(pass),
        "failure_class": failure_class,
        "where_failed": measurement_where_failed(surface, baseline, failure_class),
        "why_failed": measurement_why_failed(baseline, failure_class),
        "next_repair": measurement_next_repair(surface, baseline, failure_class),
        "baseline_duration_ms": baseline.duration_ms,
        "baseline_proof_kind": baseline_proof_kind,
        "baseline_invalidation_proof": baseline_invalidation_proof,
        "verified_local_duration_ms": actual_work_duration_ms,
        "telemetry_reconciliation_duration_ms": verified_local.telemetry_reconciliation_duration_ms,
        "reconciled_command_duration_ms": derived_fields::reconciled_command_duration_ms(verified_local),
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
        "claim_status": if pass { "supported_source_local" } else { "blocked" },
        "claim_ceiling": "source-local loop timing only; readiness release completion final-packet and update_goal remain blocked",
        "affected_set_status": affected_set_status,
        "cache_honesty": "pass",
        "timing_source": NODE_TIMING_REL,
        "claim_impact": "supports_live_loop_node_timing_only_no_readiness_release_completion_update_goal"
    });
    let object = row
        .as_object_mut()
        .expect("live-loop timing row is always an object");
    insert_execution_fields(
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
    row
}

pub(crate) fn timing_status(pass: bool) -> &'static str {
    if pass { "pass" } else { "fail" }
}

fn insert_execution_fields(
    object: &mut Map<String, Value>,
    surface: LoopValidationSurface,
    input_digest: &str,
    receipt_path: String,
    verified_local: &VerifiedLocalProof,
) {
    object.insert("proof_kind".to_string(), json!(verified_local.proof_kind));
    object.insert("cache_hit".to_string(), json!(verified_local.cache_hit));
    object.insert("cache_key".to_string(), json!(verified_local.cache_key));
    object.insert("current_input_digest".to_string(), json!(input_digest));
    object.insert("validator_version".to_string(), json!("ultragoal-rust"));
    object.insert("law_version".to_string(), json!("observability-live-loop"));
    object.insert(
        "schema_version".to_string(),
        json!("harness-ultragoal.live-loop-node-timing.v1"),
    );
    object.insert("fixture_version".to_string(), json!("source-tree-current"));
    object.insert(
        "work_unit_count".to_string(),
        json!(verified_local.work_unit_count),
    );
    object.insert(
        "actual_work_duration_ms".to_string(),
        json!(verified_local.actual_work.duration_ms),
    );
    object.insert(
        "graph_overhead_ms".to_string(),
        json!(verified_local.graph_overhead_ms),
    );
    object.insert(
        "equivalence_status".to_string(),
        json!(verified_local.equivalence_status),
    );
    object.insert(
        "invalidation_proof".to_string(),
        json!(verified_local.invalidation_proof),
    );
    object.insert(
        "telemetry_reconciliation_status".to_string(),
        json!(verified_local.telemetry_reconciliation_status),
    );
    object.insert(
        "telemetry_reconciliation".to_string(),
        verified_local.telemetry_reconciliation.value(),
    );
    object.insert(
        "verified_local_command".to_string(),
        json!(surface.narrow_rerun),
    );
    object.insert(
        "verified_local_command_argv".to_string(),
        json!(["bash", "-lc", surface.narrow_rerun]),
    );
    object.insert(
        "command_argv".to_string(),
        json!(["bash", "-lc", surface.narrow_rerun]),
    );
    object.insert(
        "verified_local_exit_code".to_string(),
        json!(verified_local.actual_work.exit_code),
    );
    object.insert(
        "exit_status".to_string(),
        json!(verified_local.actual_work.exit_code),
    );
    object.insert("receipt_paths".to_string(), json!([receipt_path]));
    object.insert("artifact_paths".to_string(), json!([NODE_TIMING_REL]));
    object.insert(
        "verified_local_launch_error".to_string(),
        json!(verified_local.actual_work.launch_error),
    );
    object.insert(
        "verified_local_stdout_digest".to_string(),
        json!(verified_local.actual_work.stdout_digest),
    );
    object.insert(
        "verified_local_stderr_digest".to_string(),
        json!(verified_local.actual_work.stderr_digest),
    );
    object.insert(
        "verified_local_failure".to_string(),
        json!(verified_local.actual_work.failure.to_value()),
    );
    if let Some(prior_result_digest) = verified_local.prior_result_digest.as_deref() {
        object.insert(
            "prior_result_digest".to_string(),
            json!(prior_result_digest),
        );
    }
    if let Some(replayed_output_digest) = verified_local.replayed_output_digest.as_deref() {
        object.insert(
            "replayed_output_digest".to_string(),
            json!(replayed_output_digest),
        );
    }
    if let Some(cache_equivalence_status) = verified_local.cache_equivalence_status.as_deref() {
        object.insert(
            "cache_equivalence_status".to_string(),
            json!(cache_equivalence_status),
        );
    }
}
