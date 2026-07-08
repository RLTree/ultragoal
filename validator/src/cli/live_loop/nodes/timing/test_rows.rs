use serde_json::json;

pub(super) fn current_timing_row(
    candidate: &str,
    input: &str,
    timing_status: &str,
    failure_class: &str,
) -> serde_json::Value {
    let exit_code = if timing_status == "pass" { 0 } else { 101 };
    let stdout_digest = digest("stdout");
    let stderr_digest = digest("stderr");
    let output_digest = digest("output");
    let result_digest = digest("result");
    let cache_key = crate::cli::live_loop::graph::verified_local_cache_key(
        "fmt_check",
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
        "telemetry_reconciliation": {"status": "pass"},
        "affected_set_status": "clean_worktree_no_affected_files"
    });
    insert_derived_fields(
        row.as_object_mut().expect("timing row object"),
        timing_status,
        exit_code,
    );
    row
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
        "runtime_execution_model".to_string(),
        json!(crate::cli::live_loop::graph::runtime_execution_model()),
    );
    object.insert("exit_status".to_string(), json!(exit_code));
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
