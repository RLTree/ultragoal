use super::super::super::timing::NODE_TIMING_REL;
use super::super::full_command;
use super::verified_work::VerifiedLocalProof;
use crate::cli::live_loop::surfaces::LoopValidationSurface;
use serde_json::{Map, Value, json};

pub(crate) fn insert(
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
    insert_command_fields(object, surface);
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
    insert_cache_replay_fields(object, verified_local);
}

fn insert_command_fields(object: &mut Map<String, Value>, surface: LoopValidationSurface) {
    object.insert(
        "verified_local_command".to_string(),
        json!(full_command::runtime_command_text(surface.narrow_rerun)),
    );
    object.insert(
        "verified_local_command_argv".to_string(),
        json!(full_command::runtime_shell_argv(surface.narrow_rerun)),
    );
    object.insert(
        "command_argv".to_string(),
        json!(full_command::runtime_shell_argv(surface.narrow_rerun)),
    );
}

fn insert_cache_replay_fields(
    object: &mut Map<String, Value>,
    verified_local: &VerifiedLocalProof,
) {
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
