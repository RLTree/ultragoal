use super::super::super::timing::NODE_TIMING_REL;
use super::super::full_command;
use super::record::MeasurementExecutionAuthority;
use super::verified_work::VerifiedLocalProof;
use crate::cli::live_loop::surfaces::{LoopValidationSurface, input_spec_for};
use serde_json::{Map, Value, json};

pub(crate) fn insert(
    object: &mut Map<String, Value>,
    surface: LoopValidationSurface,
    input_digest: &str,
    receipt_path: String,
    verified_local: &VerifiedLocalProof,
    authority: &MeasurementExecutionAuthority,
) {
    insert_scheduler_fields(object, authority);
    object.insert("proof_kind".to_string(), json!(verified_local.proof_kind));
    object.insert("cache_hit".to_string(), json!(verified_local.cache_hit));
    object.insert("cache_key".to_string(), json!(verified_local.cache_key));
    object.insert("current_input_digest".to_string(), json!(input_digest));
    object.insert(
        "validator_version".to_string(),
        json!(crate::cli::live_loop::graph::validator_version()),
    );
    object.insert(
        "law_version".to_string(),
        json!(crate::cli::live_loop::graph::law_version()),
    );
    object.insert(
        "schema_version".to_string(),
        json!(crate::cli::live_loop::graph::schema_version()),
    );
    object.insert(
        "fixture_version".to_string(),
        json!(crate::cli::live_loop::graph::fixture_version()),
    );
    object.insert(
        "runtime_execution_model".to_string(),
        json!(crate::cli::live_loop::graph::runtime_execution_model()),
    );
    insert_surface_input_spec_fields(object, surface);
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
    if surface.id == "live_loop_measurement_rust_tests" {
        object.insert(
            "verified_local_executed_test_count".to_string(),
            json!(verified_local.actual_work.executed_test_count.unwrap_or(0)),
        );
    }
    insert_cache_replay_fields(object, verified_local);
}

fn insert_surface_input_spec_fields(
    object: &mut Map<String, Value>,
    surface: LoopValidationSurface,
) {
    let Some(spec) = input_spec_for(surface.id) else {
        object.insert(
            "surface_input_spec_status".to_string(),
            json!("missing_surface_input_spec"),
        );
        return;
    };
    object.insert(
        "surface_input_spec_status".to_string(),
        json!("surface_input_spec_bound"),
    );
    object.insert(
        "surface_input_spec_node_id".to_string(),
        json!(spec.node_id),
    );
    object.insert(
        "surface_input_spec_cache_boundary".to_string(),
        json!(spec.cache_boundary_name()),
    );
    object.insert(
        "validator_authority".to_string(),
        json!(spec.validator_authority),
    );
    object.insert(
        "environment_class".to_string(),
        json!(spec.environment_class),
    );
    object.insert("cache_class".to_string(), json!(spec.cache_class));
    object.insert("claim_surface".to_string(), json!(spec.claim_surface));
    object.insert(
        "output_digest_expectation".to_string(),
        json!(spec.output_digest_expectation),
    );
}

fn insert_scheduler_fields(
    object: &mut Map<String, Value>,
    authority: &MeasurementExecutionAuthority,
) {
    object.insert(
        "graph_task_class".to_string(),
        json!(authority.graph_task_class),
    );
    object.insert(
        "execution_task_class".to_string(),
        json!(authority.execution_task_class),
    );
    object.insert(
        "execution_serial_reason".to_string(),
        json!(authority.execution_serial_reason),
    );
    object.insert("worker_count".to_string(), json!(authority.worker_count));
    object.insert("task_count".to_string(), json!(authority.task_count));
    object.insert("queue_depth".to_string(), json!(authority.queue_depth));
    object.insert("worker_state".to_string(), json!(authority.worker_state));
    object.insert("task_state".to_string(), json!(authority.task_state));
    object.insert("queue_state".to_string(), json!(authority.queue_state));
    object.insert(
        "executor_behavior".to_string(),
        json!(authority.executor_behavior),
    );
    object.insert(
        "executor_scope".to_string(),
        json!(authority.executor_scope),
    );
    object.insert(
        "parallel_write_policy".to_string(),
        json!(authority.parallel_write_policy),
    );
}

fn insert_command_fields(object: &mut Map<String, Value>, surface: LoopValidationSurface) {
    object.insert(
        "verified_local_command".to_string(),
        json!(full_command::product_command_text(surface.narrow_rerun)),
    );
    object.insert(
        "verified_local_command_argv".to_string(),
        json!(full_command::product_command_argv(surface.narrow_rerun)),
    );
    object.insert(
        "command_argv".to_string(),
        json!(full_command::product_command_argv(surface.narrow_rerun)),
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
