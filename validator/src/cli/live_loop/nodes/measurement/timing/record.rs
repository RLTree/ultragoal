use super::super::super::timing::NODE_TIMING_REL;
use super::super::{full_command::FullCommandRun, observation::TelemetryReconciliation};
use super::failure::{
    measurement_failure_class, measurement_next_repair, measurement_where_failed,
    measurement_why_failed,
};
use crate::cli::live_loop::LiveLoopCommand;
use crate::cli::live_loop::surfaces::LoopValidationSurface;
use serde_json::{Map, Value, json};

pub(crate) struct VerifiedLocalProof {
    pub(crate) proof_kind: &'static str,
    pub(crate) cache_hit: bool,
    pub(crate) cache_key: String,
    pub(crate) graph_overhead_ms: u64,
    pub(crate) actual_work: FullCommandRun,
    pub(crate) work_unit_count: u64,
    pub(crate) equivalence_status: String,
    pub(crate) invalidation_proof: String,
    pub(crate) telemetry_reconciliation_status: String,
    pub(crate) telemetry_reconciliation: TelemetryReconciliation,
    pub(crate) prior_result_digest: Option<String>,
    pub(crate) replayed_output_digest: Option<String>,
    pub(crate) cache_equivalence_status: Option<String>,
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
) -> Value {
    let actual_work_duration_ms = verified_local.actual_work.duration_ms;
    let speedup_ratio = baseline.duration_ms / actual_work_duration_ms.max(1);
    let output_digest = output_digest(verified_local);
    let result_digest = result_digest(verified_local, &output_digest);
    let failure_class = measurement_failure_class(baseline, verified_local, speedup_ratio);
    let pass = failure_class == "none";
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
        "verified_local_duration_ms": actual_work_duration_ms,
        "speedup_ratio": speedup_ratio,
        "required_speedup": "20x",
        "baseline_exit_code": baseline.exit_code,
        "baseline_launch_error": baseline.launch_error,
        "baseline_stdout_digest": baseline.stdout_digest,
        "baseline_stderr_digest": baseline.stderr_digest,
        "baseline_failure": baseline.failure.to_value(),
        "claim_name": "source-local live-loop speed claim",
        "product_behavior_observed": surface.narrow_rerun,
        "proof_surface": proof_surface(verified_local),
        "independent_reconciliation_surface": "same-candidate logs, metrics, traces, explain output, and live-loop timing receipt",
        "claim_status": if pass { "supported_source_local" } else { "blocked" },
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

fn output_digest(verified_local: &VerifiedLocalProof) -> String {
    crate::digest::bytes(
        format!(
            "stdout={};stderr={}",
            verified_local.actual_work.stdout_digest, verified_local.actual_work.stderr_digest
        )
        .as_bytes(),
    )
}

fn result_digest(verified_local: &VerifiedLocalProof, output_digest: &str) -> String {
    crate::digest::bytes(
        format!(
            "exit={};launch={};output={}",
            verified_local.actual_work.exit_code,
            verified_local.actual_work.launch_error,
            output_digest
        )
        .as_bytes(),
    )
}

fn proof_surface(verified_local: &VerifiedLocalProof) -> &'static str {
    match verified_local.proof_kind {
        "executed" => {
            "executed current-candidate command with exit status, work units, digests, and timing receipt"
        }
        "verified_cache_hit" => {
            "verified same-candidate cache replay with current input digests and equivalence proof"
        }
        _ => "invalid proof_kind; row is blocked",
    }
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
