use serde_json::json;

pub(super) const COMMAND_OBSERVATION_REL: &str =
    "validation_artifacts/observability/live-loop/commands/fmt_check-command-observation.json";

pub(super) fn row(candidate: &str, timing_status: &str, failure_class: &str) -> serde_json::Value {
    let digest = crate::digest::bytes;
    let stdout_digest = digest(b"stdout");
    let stderr_digest = digest(b"stderr");
    let output_digest = output_digest();
    let result_digest = result_digest();
    let mut row = json!({
        "node_id": "fmt_check",
        "candidate_digest": candidate,
        "tier": "hot",
        "cache_mode": "verified-local",
        "input_digest": digest(b"input"),
        "current_input_digest": digest(b"input"),
        "canonical_full_command": "cargo fmt --all --check",
        "timing_status": timing_status,
        "failure_class": failure_class,
        "validation_status": "pass",
        "validation_cache_status": "reusable",
        "observability_status": "pass",
        "speed_claim_status": "supported",
        "observability_failure_class": "none",
        "proof_kind": "executed",
        "cache_hit": false,
        "cache_key": digest(b"cache"),
        "cache_honesty": "pass",
        "telemetry_reconciliation_status": "pass",
        "exit_status": 0,
        "verified_local_launch_error": false,
        "validator_version": crate::cli::live_loop::graph::validator_version(),
        "law_version": crate::cli::live_loop::graph::law_version(),
        "schema_version": crate::cli::live_loop::graph::schema_version(),
        "fixture_version": crate::cli::live_loop::graph::fixture_version(),
        "runtime_execution_model": crate::cli::live_loop::graph::runtime_execution_model(),
        "result_digest": result_digest,
        "output_digest": output_digest,
        "verified_local_result_digest": result_digest,
        "verified_local_output_digest": output_digest,
        "verified_local_stdout_digest": stdout_digest,
        "verified_local_stderr_digest": stderr_digest,
        "baseline_stdout_digest": digest(b"baseline-stdout"),
        "baseline_stderr_digest": digest(b"baseline-stderr"),
        "claim_ceiling": crate::cli::live_loop::nodes::timing::row_authority::SOURCE_LOCAL_CLAIM_CEILING,
        "claim_impact": "supports_live_loop_node_timing_only_no_readiness_release_completion_update_goal"
    });
    let object = row.as_object_mut().expect("cache row object");
    object.insert("work_unit_count".to_string(), json!(1));
    object.insert(
        "equivalence_status".to_string(),
        json!("executed_current_candidate_not_cache_replay"),
    );
    object.insert(
        "invalidation_proof".to_string(),
        json!("cache_not_used_current_command_executed"),
    );
    object.insert(
        "telemetry_reconciliation".to_string(),
        json!({
            "status": "pass",
            "command_observation_receipt": COMMAND_OBSERVATION_REL
        }),
    );
    object.insert(
        "command_argv".to_string(),
        json!(["cargo", "fmt", "--all", "--check"]),
    );
    object.insert(
        "verified_local_command".to_string(),
        json!("cargo fmt --all --check"),
    );
    row
}

pub(super) fn output_digest() -> String {
    let stdout_digest = crate::digest::bytes(b"stdout");
    let stderr_digest = crate::digest::bytes(b"stderr");
    crate::digest::bytes(format!("stdout={stdout_digest};stderr={stderr_digest}").as_bytes())
}

pub(super) fn result_digest() -> String {
    crate::digest::bytes(format!("exit=0;launch=false;output={}", output_digest()).as_bytes())
}

pub(super) fn write_command_observation(root: &std::path::Path, row: &serde_json::Value) {
    let receipt = json!({
        "schema": "harness-ultragoal.observability-receipt.v1",
        "status": "pass",
        "candidate_digest": row["candidate_digest"],
        "receipt_path": COMMAND_OBSERVATION_REL,
        "operation": "loop.measure.fmt_check",
        "event": {
            "status": "pass",
            "candidate_digest": row["candidate_digest"],
            "artifact_path": crate::cli::live_loop::nodes::timing::NODE_TIMING_REL,
            "operation": "loop.measure.fmt_check",
            "command": "cargo",
            "subcommand": "fmt --all --check"
        },
        "command_result_authority": {
            "authority": "live_loop_command_observation_actual_work",
            "exit_status": 0,
            "launch_error": false,
            "stdout_digest": crate::digest::bytes(b"stdout"),
            "stderr_digest": crate::digest::bytes(b"stderr"),
            "output_digest": output_digest(),
            "result_digest": result_digest(),
            "executed_test_count": serde_json::Value::Null
        }
    });
    crate::json_boundary::write_json(&root.join(COMMAND_OBSERVATION_REL), &receipt)
        .expect("command observation receipt");
}

pub(super) trait WithValue {
    fn with_value(self, key: &str, value: serde_json::Value) -> Self;
}

impl WithValue for serde_json::Value {
    fn with_value(mut self, key: &str, value: serde_json::Value) -> Self {
        self.as_object_mut()
            .expect("timing row object")
            .insert(key.to_string(), value);
        self
    }
}
