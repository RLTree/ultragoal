use super::super::super::{LiveLoopAction, LiveLoopCommand};
use serde_json::json;
use std::path::{Path, PathBuf};

const NODE_TIMING_RECEIPT: &str = "validation_artifacts/observability/live-loop-node-timing.json";

fn command(node_id: Option<&str>, receipt: PathBuf) -> LiveLoopCommand {
    LiveLoopCommand {
        action: LiveLoopAction::Measure,
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: None,
        receipt,
        node_id: node_id.map(str::to_string),
        measure_all: false,
    }
}

fn node_timing_receipt_arg() -> PathBuf {
    Path::new(NODE_TIMING_RECEIPT).to_path_buf()
}

fn node_timing_receipt_path(root: &std::path::Path) -> PathBuf {
    crate::output_path::literal_claim_artifact_path(
        root,
        NODE_TIMING_RECEIPT,
        "node timing receipt",
    )
}

#[test]
fn live_loop_measure_rejects_missing_or_unknown_node_id() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-measure-errors");
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");

    let missing = super::super::super::run(&root, &command(None, node_timing_receipt_arg()))
        .expect_err("missing node rejected");
    assert!(
        missing.contains("loop measure requires --node <id>"),
        "{missing}"
    );

    let unknown = super::super::super::run(
        &root,
        &command(Some("unknown_node"), node_timing_receipt_arg()),
    )
    .expect_err("unknown node rejected");
    assert!(unknown.contains("unknown live-loop node"), "{unknown}");

    std::fs::remove_dir_all(root).expect("cleanup measure errors");
}

#[test]
fn live_loop_measure_reports_digest_receipt_and_launch_failures() {
    let missing_manifest =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-measure-digest");
    std::fs::create_dir_all(&missing_manifest).expect("missing manifest root");
    let digest_err = super::super::super::run(
        &missing_manifest,
        &command(Some("changed_files"), node_timing_receipt_arg()),
    )
    .expect_err("missing manifest rejected");
    assert!(
        digest_err.contains("plugin-manifest-draft.json"),
        "{digest_err}"
    );
    std::fs::remove_dir_all(missing_manifest).expect("cleanup missing manifest");

    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-measure-receipt");
    std::fs::create_dir_all(&root).expect("receipt root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let receipt_err = super::super::super::run(
        &root,
        &command(Some("changed_files"), root.join(NODE_TIMING_RECEIPT)),
    )
    .expect_err("absolute receipt rejected");
    assert!(
        receipt_err.contains("root-relative claim artifact path"),
        "{receipt_err}"
    );

    let surface = crate::cli::live_loop::surfaces::surface_by_id("changed_files").expect("surface");
    let launch_failure = super::full_command::run_full_command_with_shell(
        &root,
        surface,
        "ultragoal-missing-shell-for-test",
    );
    assert!(!launch_failure.status_success);
    assert!(launch_failure.launch_error);
    assert_eq!(
        super::node_timing_receipt::measurement_failure_class(&launch_failure, 0),
        "canonical_full_command_launch_failed"
    );

    std::fs::remove_dir_all(root).expect("cleanup receipt root");
}

#[test]
fn live_loop_measure_writes_current_node_timing_from_real_command_surface() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-measure-command");
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let receipt = node_timing_receipt_path(&root);
    let command = command(Some("changed_files"), node_timing_receipt_arg());

    let first_code = super::super::super::run(&root, &command).expect("first measure command");
    assert_eq!(first_code, 0);
    let first_timing = crate::json_boundary::read_json(&receipt).expect("first timing receipt");
    let first_rows = first_timing["nodes"].as_array().expect("first nodes");
    assert_eq!(first_rows.len(), 1);
    assert_eq!(first_rows[0]["node_id"], "changed_files");

    crate::json_boundary::write_json(
        &receipt,
        &json!({
            "nodes": [{
                "node_id": "stale_node",
                "candidate_digest": "sha256:stale",
                "tier": "hot",
                "cache_mode": "verified-local"
            }, {
                "node_id": "existing_current_node",
                "candidate_digest": candidate,
                "tier": "hot",
                "cache_mode": "verified-local"
            }]
        }),
    )
    .expect("stale timing");

    let code = super::super::super::run(&root, &command).expect("measure command");
    assert_eq!(code, 0);
    let timing = crate::json_boundary::read_json(&receipt).expect("timing receipt");
    assert_eq!(
        timing["schema"],
        "harness-ultragoal.live-loop-node-timing.v1"
    );
    assert_eq!(timing["candidate_digest"], candidate);
    let rows = timing["nodes"].as_array().expect("nodes");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["node_id"], "changed_files");
    assert_eq!(rows[1]["node_id"], "existing_current_node");
    assert_eq!(
        rows[0]["canonical_full_command"],
        "git status --short --untracked-files=all"
    );
    assert_eq!(rows[0]["baseline_exit_code"], 0);
    assert_eq!(rows[0]["failure_class"], "none");
    assert_eq!(rows[0]["affected_set_status"], "changed_files_digest_bound");

    std::fs::remove_dir_all(root).expect("cleanup measure command");
}

#[test]
fn live_loop_measure_projects_empty_affected_set_and_failure_classes() {
    assert_eq!(super::node_timing_receipt::timing_status(true), "pass");
    assert_eq!(super::node_timing_receipt::timing_status(false), "fail");
    assert_eq!(
        super::node_timing_receipt::affected_set_status(&[]),
        "clean_worktree_no_affected_files"
    );
    assert_eq!(
        super::node_timing_receipt::affected_set_status(&[" M validator/src/lib.rs".to_string()]),
        "changed_files_digest_bound"
    );

    let failed_baseline = super::full_command::FullCommandRun {
        exit_code: 1,
        status_success: false,
        launch_error: false,
        duration_ms: 50,
        stdout_digest: "sha256:stdout".to_string(),
        stderr_digest: "sha256:stderr".to_string(),
    };
    assert_eq!(
        super::node_timing_receipt::measurement_failure_class(&failed_baseline, 50),
        "canonical_full_command_failed"
    );

    let slow_baseline = super::full_command::FullCommandRun {
        exit_code: 0,
        status_success: true,
        launch_error: false,
        duration_ms: 50,
        stdout_digest: "sha256:stdout".to_string(),
        stderr_digest: "sha256:stderr".to_string(),
    };
    assert_eq!(
        super::node_timing_receipt::measurement_failure_class(&slow_baseline, 1),
        "live_loop_speedup_target_missed"
    );
    assert_eq!(
        super::node_timing_receipt::measurement_failure_class(&slow_baseline, 20),
        "none"
    );
}

#[test]
fn live_loop_measure_marks_verified_local_speedup_as_pass() {
    let command = command(Some("changed_files"), node_timing_receipt_arg());
    let baseline = super::full_command::FullCommandRun {
        exit_code: 0,
        status_success: true,
        launch_error: false,
        duration_ms: 200,
        stdout_digest: "sha256:stdout".to_string(),
        stderr_digest: "sha256:stderr".to_string(),
    };
    let row = super::node_timing_receipt::node_timing_row(
        crate::cli::live_loop::surfaces::surface_by_id("changed_files").expect("surface"),
        &command,
        "sha256:candidate",
        "sha256:changed",
        "sha256:audit",
        "sha256:input",
        &baseline,
        10,
        "changed_files_digest_bound",
    );

    assert_eq!(row["timing_status"], "pass");
    assert_eq!(row["failure_class"], "none");
    assert_eq!(
        row["claim_impact"],
        "supports_live_loop_node_timing_only_no_readiness_release_completion_update_goal"
    );
}
