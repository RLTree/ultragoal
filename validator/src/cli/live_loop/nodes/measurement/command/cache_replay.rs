use super::fixtures::{command, live_loop_timing_receipt_arg, live_loop_timing_receipt_path};
use crate::cli::live_loop::changed_inputs::ChangedInputs;
use crate::cli::live_loop::surfaces::{LoopValidationSurface, input_spec_for, surface_by_id};
use serde_json::json;
use std::path::Path;

const COMMAND_OBSERVATION_REL: &str = "validation_artifacts/observability/changed-files.json";

#[test]
fn live_loop_measure_replays_current_input_cache_row_into_timing_output() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("cache-replay");
    std::fs::create_dir_all(&root).expect("root");
    std::process::Command::new("git")
        .arg("init")
        .current_dir(&root)
        .output()
        .expect("git init");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");

    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let command = command(Some("changed_files"), live_loop_timing_receipt_arg());
    let surface = surface_by_id("changed_files").expect("changed files surface");
    let inputs = ChangedInputs::collect(&root, &candidate, &command.tier, &command.cache_mode);
    let input_digest = super::super::super::super::graph::surface_input_digest(
        surface,
        &candidate,
        inputs.surface_digest(surface),
        &inputs.audit_context_digest,
    );
    let cache_key = super::super::super::super::graph::verified_local_cache_key(
        surface,
        &input_digest,
        &command.tier,
        &command.cache_mode,
    );
    let receipt = live_loop_timing_receipt_path(&root);
    std::fs::create_dir_all(receipt.parent().expect("receipt parent")).expect("receipt dir");
    let cache_row = cache_row(&root, surface, &candidate, &input_digest, &cache_key);
    crate::json_boundary::write_json(&receipt, &json!({"nodes": [cache_row.clone()]}))
        .expect("prior timing row");
    let replay_store = super::super::cache_replay::ReplayStore::load(&root, &command);

    let row = super::super::measure_surface(
        &root,
        &command,
        surface,
        &candidate,
        &inputs,
        &replay_store,
        super::super::timing::receipt::affected_set_status(inputs.changed_file_count),
        super::super::ObservationMode::FullRoundtrip,
    );

    assert_eq!(row["proof_kind"], "verified_cache_hit");
    assert_eq!(row["cache_hit"], true);
    assert_eq!(row["work_unit_count"], 0);
    assert_eq!(row["baseline_proof_kind"], "verified_baseline_reuse");
    assert_eq!(
        row["baseline_invalidation_proof"],
        "baseline_reused_from_verified_current_input_timing_row"
    );
    assert_eq!(
        row["equivalence_status"],
        "verified_same_candidate_cache_replay"
    );
    assert_eq!(
        row["invalidation_proof"],
        "cache_key_current_input_digest_command_contract_runtime_model_versions_and_environment_matched"
    );
    assert_eq!(row["prior_result_digest"], result_digest());
    assert_eq!(row["replayed_output_digest"], output_digest());
    assert_eq!(row["cache_equivalence_status"], "pass");
    assert_eq!(row["candidate_digest"], candidate);
    assert_eq!(row["input_digest"], input_digest);
    assert_eq!(row["cache_key"], cache_key);

    let mut substituted_receipt =
        crate::json_boundary::read_json(&root.join(COMMAND_OBSERVATION_REL))
            .expect("bound command observation receipt");
    substituted_receipt["command_identity"]["node_id"] = json!("substituted-node");
    crate::json_boundary::write_json(&root.join(COMMAND_OBSERVATION_REL), &substituted_receipt)
        .expect("substituted command observation receipt");
    let substituted_digest = crate::digest::canonical_json(&substituted_receipt);
    let mut substituted_row = cache_row;
    substituted_row["command_observation_receipt_digest"] = json!(substituted_digest);
    substituted_row["telemetry_reconciliation"]["command_observation_receipt_digest"] =
        substituted_row["command_observation_receipt_digest"].clone();
    crate::json_boundary::write_json(&receipt, &json!({"nodes": [substituted_row]}))
        .expect("digest-bound semantic substitution row");
    let substituted_store = super::super::cache_replay::ReplayStore::load(&root, &command);
    let result = super::super::measure_surface(
        &root,
        &command,
        surface,
        &candidate,
        &inputs,
        &substituted_store,
        super::super::timing::receipt::affected_set_status(inputs.changed_file_count),
        super::super::ObservationMode::LoopRunSnapshot,
    );
    assert_eq!(result["proof_kind"], "executed");
    assert_eq!(result["cache_hit"], false);
    assert_eq!(result["work_unit_count"], 1);
    assert_eq!(
        result["invalidation_proof"],
        "cache_not_used_current_command_executed"
    );

    std::fs::remove_dir_all(root).expect("cleanup measure cache replay");
}

fn cache_row(
    root: &Path,
    surface: LoopValidationSurface,
    candidate: &str,
    input_digest: &str,
    cache_key: &str,
) -> serde_json::Value {
    let input_spec = input_spec_for("changed_files").expect("changed_files input spec");
    let command_argv = super::super::full_command::product_command_argv(surface.narrow_rerun);
    let verified_local_command =
        super::super::full_command::product_command_text(surface.narrow_rerun);
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

fn stdout_digest() -> String {
    crate::digest::bytes(b"stdout")
}

fn stderr_digest() -> String {
    crate::digest::bytes(b"stderr")
}

fn output_digest() -> String {
    crate::digest::bytes(
        format!("stdout={};stderr={}", stdout_digest(), stderr_digest()).as_bytes(),
    )
}

fn result_digest() -> String {
    crate::digest::bytes(format!("exit=0;launch=false;output={}", output_digest()).as_bytes())
}

trait WithValue {
    fn with_value(self, key: &str, value: serde_json::Value) -> Self;
}

impl WithValue for serde_json::Value {
    fn with_value(mut self, key: &str, value: serde_json::Value) -> Self {
        self.as_object_mut().unwrap().insert(key.to_string(), value);
        self
    }
}
