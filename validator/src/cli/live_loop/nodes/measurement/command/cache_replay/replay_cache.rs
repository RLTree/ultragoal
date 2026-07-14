use super::*;

pub(crate) fn cache_row(
    root: &Path,
    surface: LoopValidationSurface,
    candidate: &str,
    input_digest: &str,
    cache_key: &str,
) -> serde_json::Value {
    let input_spec = input_spec_for("changed_files").expect("changed_files input spec");
    let command_argv =
        crate::cli::live_loop::nodes::measurement::full_command::product_command_argv(
            surface.narrow_rerun,
        );
    let verified_local_command =
        crate::cli::live_loop::nodes::measurement::full_command::product_command_text(
            surface.narrow_rerun,
        );
    let command_receipt = json!({
        "schema": "harness-ultragoal.observe-receipt.v1",
        "status": "pass",
        "candidate_digest": candidate,
        "run_id": "run-changed-files-cache-replay",
        "correlation_id": "corr-changed-files-cache-replay",
        "trace_id": "trace-changed-files-cache-replay",
        "command_identity": {
            "node_id": surface.id,
            "canonical_full_command": surface.canonical_full_command,
            "verified_local_command": verified_local_command,
            "command_argv": command_argv
        },
        "process_result_authority": {
            "exit_status": 0,
            "status_success": true,
            "launch_error": false,
            "duration_ms": 100,
            "work_unit_count": 1,
            "stdout_digest": stdout_digest(),
            "stderr_digest": stderr_digest(),
            "output_digest": output_digest(),
            "result_digest": result_digest(),
            "redaction_status": "pass",
            "bounded_output_status": "digest_only_raw_output_not_retained",
            "claim_ceiling": "source_local_command_result_authority_only"
        }
    });
    crate::json_boundary::write_json(&root.join(COMMAND_OBSERVATION_REL), &command_receipt)
        .expect("materialized command observation receipt");
    let command_receipt_digest = crate::digest::canonical_json(&command_receipt);
    json!({
        "node_id": "changed_files",
        "candidate_digest": candidate,
        "tier": "hot",
        "cache_mode": "verified-local",
        "input_digest": input_digest,
        "current_input_digest": input_digest,
        "canonical_full_command": "git status --short --untracked-files=all",
        "proof_kind": "executed",
        "cache_hit": false,
        "cache_key": cache_key,
        "cache_honesty": "pass",
        "validator_version": crate::cli::live_loop::graph::validator_version(),
        "law_version": crate::cli::live_loop::graph::law_version(),
        "schema_version": crate::cli::live_loop::graph::schema_version(),
        "fixture_version": crate::cli::live_loop::graph::fixture_version(),
        "work_unit_count": 1,
        "actual_work_duration_ms": 100,
        "graph_overhead_ms": 1,
        "equivalence_status": "executed_current_candidate_not_cache_replay",
        "invalidation_proof": "input_digest_and_candidate_checked",
        "telemetry_reconciliation_status": "pass",
        "telemetry_reconciliation": {"status": "pass"},
        "verified_local_command_argv": ["git", "status", "--short", "--untracked-files=all"],
        "command_argv": ["git", "status", "--short", "--untracked-files=all"],
        "verified_local_exit_code": 0,
        "exit_status": 0,
        "verified_local_launch_error": false,
        "baseline_duration_ms": 200,
        "baseline_exit_code": 0,
        "baseline_launch_error": false,
        "baseline_stdout_digest": stdout_digest(),
        "baseline_stderr_digest": stderr_digest(),
        "baseline_failure": {},
        "receipt_paths": [live_loop_timing_receipt_arg()],
        "artifact_paths": ["validation_artifacts/observability/live-loop-node-timing.json"],
        "verified_local_stdout_digest": stdout_digest(),
        "verified_local_stderr_digest": stderr_digest(),
        "output_digest": output_digest(),
        "verified_local_output_digest": output_digest(),
        "result_digest": result_digest(),
        "verified_local_result_digest": result_digest()
    })
    .with_value(
        "runtime_execution_model",
        json!(crate::cli::live_loop::graph::runtime_execution_model()),
    )
    .with_value("validation_status", json!("pass"))
    .with_value("validation_cache_status", json!("reusable"))
    .with_value("observability_status", json!("pass"))
    .with_value("speed_claim_status", json!("supported"))
    .with_value("observability_failure_class", json!("none"))
    .with_value(
        "telemetry_reconciliation",
        json!({
            "status": "pass",
            "command_observation_receipt": COMMAND_OBSERVATION_REL,
            "command_observation_receipt_digest": command_receipt_digest,
            "process_result_digest": result_digest()
        }),
    )
    .with_value(
        "command_observation_receipt",
        json!(COMMAND_OBSERVATION_REL),
    )
    .with_value(
        "command_observation_receipt_digest",
        json!(command_receipt_digest),
    )
    .with_value("process_result_digest", json!(result_digest()))
    .with_value("verified_local_command", json!(verified_local_command))
    .with_value(
        "surface_input_spec_status",
        json!("surface_input_spec_bound"),
    )
    .with_value("surface_input_spec_node_id", json!(input_spec.node_id))
    .with_value(
        "surface_input_spec_cache_boundary",
        json!(input_spec.cache_boundary_name()),
    )
    .with_value("validator_authority", json!(input_spec.validator_authority))
    .with_value("environment_class", json!(input_spec.environment_class))
    .with_value("cache_class", json!(input_spec.cache_class))
    .with_value("claim_surface", json!(input_spec.claim_surface))
    .with_value(
        "output_digest_expectation",
        json!(input_spec.output_digest_expectation),
    )
    .with_value("graph_task_class", json!("pure_read_parallel"))
    .with_value("execution_task_class", json!("pure_read_parallel"))
    .with_value("execution_serial_reason", json!("none"))
    .with_value("worker_count", json!(1))
    .with_value("task_count", json!(1))
    .with_value("queue_depth", json!(1))
    .with_value("worker_state", json!("single_surface_measurement_worker"))
    .with_value("task_state", json!("surface_measurement_completed"))
    .with_value(
        "queue_state",
        json!("deterministic_measurement_batch_order"),
    )
    .with_value(
        "executor_behavior",
        json!("measure_surface_invokes_one_node_command_at_a_time"),
    )
    .with_value(
        "executor_scope",
        json!("source_local_custom_tooling_prerequisite_measurement_runner"),
    )
    .with_value(
        "parallel_write_policy",
        json!("no_shared_validation_artifact_parallel_write"),
    )
    .with_value("telemetry_reconciliation_duration_ms", json!(3))
    .with_value("reconciled_command_duration_ms", json!(104))
    .with_value("product_latency_ms", json!(101))
}

pub(crate) fn stdout_digest() -> String {
    crate::digest::bytes(b"stdout")
}

pub(crate) fn stderr_digest() -> String {
    crate::digest::bytes(b"stderr")
}

pub(crate) fn output_digest() -> String {
    crate::digest::bytes(
        format!("stdout={};stderr={}", stdout_digest(), stderr_digest()).as_bytes(),
    )
}

pub(crate) fn result_digest() -> String {
    crate::digest::bytes(format!("exit=0;launch=false;output={}", output_digest()).as_bytes())
}

pub(crate) trait WithValue {
    fn with_value(self, key: &str, value: serde_json::Value) -> Self;
}

impl WithValue for serde_json::Value {
    fn with_value(mut self, key: &str, value: serde_json::Value) -> Self {
        self.as_object_mut().unwrap().insert(key.to_string(), value);
        self
    }
}
