#![allow(dead_code)]

use serde_json::json;

pub(super) const FMT_COMMAND_OBSERVATION_REL: &str =
    "validation_artifacts/observability/live-loop/commands/fmt_check-command-observation.json";

pub(super) fn current_timing_row(
    candidate: &str,
    input: &str,
    timing_status: &str,
    failure_class: &str,
) -> serde_json::Value {
    let exit_code = if timing_status == "pass" { 0 } else { 101 };
    let stdout_digest = digest("stdout");
    let stderr_digest = digest("stderr");
    let output_digest = output_digest(&stdout_digest, &stderr_digest);
    let result_digest = expected_result_digest(exit_code, false, &output_digest);
    let surface = crate::cli::live_loop::surfaces::surface_by_id("fmt_check").expect("fmt surface");
    let cache_key = crate::cli::live_loop::graph::verified_local_cache_key(
        surface,
        input,
        "hot",
        "verified-local",
    );
    let mut row = json!({
        "node_id": "fmt_check",
        "candidate_digest": candidate,
        "tier": "hot",
        "cache_mode": "verified-local",
        "input_digest": input,
        "current_input_digest": input,
        "canonical_full_command": "cargo fmt --all --check",
        "timing_status": timing_status,
        "failure_class": failure_class,
        "baseline_exit_code": exit_code,
        "baseline_launch_error": false,
        "cache_honesty": "pass",
        "baseline_duration_ms": 1000,
        "verified_local_duration_ms": 2,
        "proof_kind": "executed",
        "cache_hit": false,
        "cache_key": cache_key,
        "validator_version": crate::cli::live_loop::graph::validator_version(),
        "law_version": crate::cli::live_loop::graph::law_version(),
        "schema_version": crate::cli::live_loop::graph::schema_version(),
        "fixture_version": crate::cli::live_loop::graph::fixture_version(),
        "work_unit_count": 1,
        "actual_work_duration_ms": 2,
        "graph_overhead_ms": 1,
        "equivalence_status": "executed_current_candidate_not_cache_replay",
        "invalidation_proof": "cache_not_used_current_command_executed",
        "telemetry_reconciliation_status": "pass",
        "verified_local_command": "cargo fmt --all --check",
        "verified_local_command_argv": ["cargo", "fmt", "--all", "--check"],
        "verified_local_stdout_digest": stdout_digest,
        "verified_local_stderr_digest": stderr_digest,
        "verified_local_exit_code": exit_code,
        "output_digest": output_digest,
        "result_digest": result_digest,
        "verified_local_output_digest": output_digest,
        "verified_local_result_digest": result_digest,
        "where_failed": "loop.measure.fmt_check.canonical_full_command",
        "why_failed": "canonical full command exited nonzero while measuring live-loop node",
        "next_repair": "run cargo fmt and rerun loop measure",
        "affected_set_status": "clean_worktree_no_affected_files"
    });
    insert_derived_fields(
        row.as_object_mut().expect("timing row object"),
        timing_status,
        exit_code,
    );
    row
}

fn output_digest(stdout_digest: &str, stderr_digest: &str) -> String {
    crate::digest::bytes(format!("stdout={stdout_digest};stderr={stderr_digest}").as_bytes())
}

pub(super) fn expected_output_digest() -> String {
    let stdout_digest = digest("stdout");
    let stderr_digest = digest("stderr");
    output_digest(&stdout_digest, &stderr_digest)
}

pub(super) fn expected_result_digest(
    exit_code: i32,
    launch_error: bool,
    output_digest: &str,
) -> String {
    crate::digest::bytes(
        format!("exit={exit_code};launch={launch_error};output={output_digest}").as_bytes(),
    )
}

fn insert_derived_fields(
    object: &mut serde_json::Map<String, serde_json::Value>,
    timing_status: &str,
    exit_code: i32,
) {
    object.insert(
        "baseline_proof_kind".to_string(),
        json!("executed_same_command_reuse"),
    );
    object.insert(
        "baseline_invalidation_proof".to_string(),
        json!(
            "baseline_reused_from_executed_narrow_command_because_canonical_full_command_matches"
        ),
    );
    object.insert(
        "command_argv".to_string(),
        json!(["cargo", "fmt", "--all", "--check"]),
    );
    object.insert(
        "verified_local_command_argv".to_string(),
        json!(["cargo", "fmt", "--all", "--check"]),
    );
    insert_scheduler_fields(object);
    object.insert(
        "runtime_execution_model".to_string(),
        json!(crate::cli::live_loop::graph::runtime_execution_model()),
    );
    object.insert("exit_status".to_string(), json!(exit_code));
    object.insert("verified_local_launch_error".to_string(), json!(false));
    object.insert("telemetry_reconciliation_duration_ms".to_string(), json!(3));
    object.insert("reconciled_command_duration_ms".to_string(), json!(6));
    object.insert("product_latency_ms".to_string(), json!(3));
    object.insert(
        "receipt_paths".to_string(),
        json!([super::super::NODE_TIMING_REL]),
    );
    object.insert(
        "artifact_paths".to_string(),
        json!([super::super::NODE_TIMING_REL]),
    );
    object.insert(
        "validation_status".to_string(),
        json!(if timing_status == "pass" {
            "pass"
        } else {
            "fail"
        }),
    );
    object.insert(
        "validation_cache_status".to_string(),
        json!(if timing_status == "pass" {
            "reusable"
        } else {
            "not_reusable"
        }),
    );
    object.insert("observability_status".to_string(), json!("pass"));
    object.insert(
        "speed_claim_status".to_string(),
        json!(if timing_status == "pass" {
            "supported"
        } else {
            "withheld"
        }),
    );
    object.insert("observability_failure_class".to_string(), json!("none"));
    object.insert(
        "telemetry_reconciliation".to_string(),
        json!({
            "status": "pass",
            "command_observation_receipt": FMT_COMMAND_OBSERVATION_REL
        }),
    );
    object.insert(
        "claim_ceiling".to_string(),
        json!(crate::cli::live_loop::nodes::timing::row_authority::SOURCE_LOCAL_CLAIM_CEILING),
    );
    object.insert(
        "claim_impact".to_string(),
        json!("supports_live_loop_node_timing_only_no_readiness_release_completion_update_goal"),
    );
}

fn insert_scheduler_fields(object: &mut serde_json::Map<String, serde_json::Value>) {
    object.insert("graph_task_class".to_string(), json!("pure_read_parallel"));
    object.insert(
        "execution_task_class".to_string(),
        json!("pure_read_parallel"),
    );
    object.insert("execution_serial_reason".to_string(), json!("none"));
    object.insert("worker_count".to_string(), json!(1));
    object.insert("task_count".to_string(), json!(1));
    object.insert("queue_depth".to_string(), json!(1));
    object.insert(
        "worker_state".to_string(),
        json!("single_surface_measurement_worker"),
    );
    object.insert(
        "task_state".to_string(),
        json!("surface_measurement_completed"),
    );
    object.insert(
        "queue_state".to_string(),
        json!("deterministic_measurement_batch_order"),
    );
    object.insert(
        "executor_behavior".to_string(),
        json!("measure_surface_invokes_one_node_command_at_a_time"),
    );
    object.insert(
        "executor_scope".to_string(),
        json!("source_local_custom_tooling_prerequisite_measurement_runner"),
    );
    object.insert(
        "parallel_write_policy".to_string(),
        json!("no_shared_validation_artifact_parallel_write"),
    );
}

pub(super) fn fmt_input(candidate: &str, changed: &str, context: &str) -> String {
    crate::cli::live_loop::graph::surface_input_digest(
        crate::cli::live_loop::surfaces::surface_by_id("fmt_check").expect("fmt surface"),
        candidate,
        changed,
        context,
    )
}

pub(super) fn changed_inputs(
    changed: &str,
    context: &str,
) -> crate::cli::live_loop::changed_inputs::ChangedInputs {
    crate::cli::live_loop::changed_inputs::ChangedInputs::for_tests(changed, context)
}

pub(super) fn digest(label: &str) -> String {
    crate::digest::bytes(label.as_bytes())
}
