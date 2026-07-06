use super::fixtures::{command, live_loop_timing_receipt_arg, live_loop_timing_receipt_path};
use crate::cli::live_loop::surfaces::surface_by_id;
use serde_json::json;

#[test]
fn live_loop_measure_replays_same_candidate_cache_row_into_timing_output() {
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
    let changed_files = super::super::changed_files(&root);
    let changed_files_digest = crate::digest::bytes(changed_files.join("\n").as_bytes());
    let audit_context_digest = crate::digest::bytes(
        format!(
            "{}:{}:{}:{}",
            candidate, command.tier, command.cache_mode, changed_files_digest
        )
        .as_bytes(),
    );
    let input_digest = super::super::super::super::graph::surface_input_digest(
        surface,
        &candidate,
        &changed_files_digest,
        &audit_context_digest,
    );
    let cache_key = super::super::super::super::graph::verified_local_cache_key(
        surface.id,
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

    let row = super::super::measure_surface(
        &root,
        &command,
        surface,
        &candidate,
        &changed_files_digest,
        &audit_context_digest,
        super::super::timing::receipt::affected_set_status(&changed_files),
    );

    assert_eq!(row["proof_kind"], "verified_cache_hit");
    assert_eq!(row["cache_hit"], true);
    assert_eq!(row["work_unit_count"], 0);
    assert_eq!(row["baseline_proof_kind"], "verified_baseline_reuse");
    assert_eq!(
        row["baseline_invalidation_proof"],
        "baseline_reused_from_same_candidate_current_input_timing_row"
    );
    assert_eq!(
        row["equivalence_status"],
        "verified_same_candidate_cache_replay"
    );
    assert_eq!(
        row["invalidation_proof"],
        "cache_key_current_input_digest_command_versions_and_candidate_row_matched"
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
        "validator_version": "ultragoal-rust",
        "law_version": "observability-live-loop",
        "schema_version": "harness-ultragoal.live-loop-node-timing.v1",
        "fixture_version": "source-tree-current",
        "work_unit_count": 1,
        "actual_work_duration_ms": 100,
        "graph_overhead_ms": 1,
        "equivalence_status": "executed_current_candidate_not_cache_replay",
        "invalidation_proof": "input_digest_and_candidate_checked",
        "telemetry_reconciliation_status": "pass",
        "verified_local_command_argv": ["bash", "-lc", "git status --short --untracked-files=all"],
        "command_argv": ["bash", "-lc", "git status --short --untracked-files=all"],
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
