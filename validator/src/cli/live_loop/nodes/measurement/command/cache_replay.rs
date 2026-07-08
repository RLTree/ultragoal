use super::fixtures::{command, live_loop_timing_receipt_arg, live_loop_timing_receipt_path};
use crate::cli::live_loop::changed_inputs::ChangedInputs;
use crate::cli::live_loop::surfaces::surface_by_id;
use serde_json::json;

#[test]
fn live_loop_measure_replays_current_input_cache_row_into_timing_output() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-measure-cache-replay",
    );
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
    crate::json_boundary::write_json(
        &receipt,
        &json!({"nodes": [cache_row(&candidate, &input_digest, &cache_key)]}),
    )
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

    std::fs::remove_dir_all(root).expect("cleanup measure cache replay");
}

fn cache_row(candidate: &str, input_digest: &str, cache_key: &str) -> serde_json::Value {
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
        self.as_object_mut()
            .expect("timing row object")
            .insert(key.to_string(), value);
        self
    }
}
