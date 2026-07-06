use super::fixtures::{
    command, full_command_run, live_loop_timing_receipt_arg, live_loop_timing_receipt_path,
    verified_local_proof,
};
use crate::self_tests::boundaries::workspace_fixtures::temp_root;
use serde_json::json;

#[test]
fn live_loop_measure_writes_current_node_timing_from_real_command_surface() {
    let root = temp_root("live-loop-measure-command");
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let receipt = live_loop_timing_receipt_path(&root);
    let command = command(Some("changed_files"), live_loop_timing_receipt_arg());

    let first_code = crate::cli::live_loop::run(&root, &command).expect("first measure command");
    assert_eq!(first_code, 1);
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

    let code = crate::cli::live_loop::run(&root, &command).expect("measure command");
    assert_eq!(code, 1);
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
    let failure_class = rows[0]["failure_class"].as_str().expect("failure class");
    assert!(
        [
            "live_loop_telemetry_reconciliation_missing",
            "live_loop_speedup_target_missed"
        ]
        .contains(&failure_class),
        "{failure_class}"
    );
    assert_eq!(rows[0]["proof_kind"], "executed");
    assert_eq!(rows[0]["cache_hit"], false);
    assert_eq!(rows[0]["work_unit_count"], 1);
    let telemetry_status = rows[0]["telemetry_reconciliation_status"]
        .as_str()
        .expect("telemetry status");
    assert!(
        matches!(
            telemetry_status,
            "query_or_explain_reconciliation_failed" | "pass"
        ),
        "{telemetry_status}"
    );
    assert_ne!(telemetry_status, "missing_command_telemetry");
    let telemetry = &rows[0]["telemetry_reconciliation"];
    assert_eq!(
        telemetry["command_observation"]["event"]["operation"],
        "loop.measure.changed_files"
    );
    assert!(
        telemetry["command_observation_receipt"]
            .as_str()
            .expect("command observation receipt")
            .contains("changed_files-command-observation.json")
    );
    assert!(
        rows[0]["result_digest"]
            .as_str()
            .expect("result digest")
            .starts_with("sha256:")
    );
    assert_eq!(
        rows[0]["result_digest"],
        rows[0]["verified_local_result_digest"]
    );
    assert_eq!(
        rows[0]["output_digest"],
        rows[0]["verified_local_output_digest"]
    );
    assert!(
        rows[0]["verified_local_result_digest"]
            .as_str()
            .expect("result digest")
            .starts_with("sha256:")
    );
    let affected_set_status = rows[0]["affected_set_status"]
        .as_str()
        .expect("affected set status");
    assert!(
        matches!(
            affected_set_status,
            "clean_worktree_no_affected_files" | "changed_files_digest_bound"
        ),
        "{affected_set_status}"
    );

    std::fs::remove_dir_all(root).expect("cleanup measure command");
}

#[test]
fn live_loop_measure_projects_empty_affected_set_and_failure_classes() {
    assert_eq!(super::super::timing::record::timing_status(true), "pass");
    assert_eq!(super::super::timing::record::timing_status(false), "fail");
    assert_eq!(
        super::super::timing::receipt::affected_set_status(&[]),
        "clean_worktree_no_affected_files"
    );
    assert_eq!(
        super::super::timing::receipt::affected_set_status(
            &[" M validator/src/lib.rs".to_string()]
        ),
        "changed_files_digest_bound"
    );

    let failed_baseline = full_command_run(1, false, 50);
    assert_eq!(
        super::super::timing::failure::measurement_failure_class(
            &failed_baseline,
            &verified_local_proof(0, true, 1, "pass"),
            50,
        ),
        "canonical_full_command_failed"
    );

    let slow_baseline = full_command_run(0, true, 50);
    assert_eq!(
        super::super::timing::failure::measurement_failure_class(
            &slow_baseline,
            &verified_local_proof(0, true, 1, "pass"),
            1,
        ),
        "live_loop_speedup_target_missed"
    );
    assert_eq!(
        super::super::timing::failure::measurement_failure_class(
            &slow_baseline,
            &verified_local_proof(0, true, 1, "missing_query_reconciliation"),
            20,
        ),
        "live_loop_telemetry_reconciliation_missing"
    );
    assert_eq!(
        super::super::timing::failure::measurement_failure_class(
            &slow_baseline,
            &verified_local_proof(0, true, 1, "pass"),
            20,
        ),
        "none"
    );
}

#[test]
fn live_loop_measure_rejects_speedup_without_telemetry_reconciliation() {
    let command = command(Some("changed_files"), live_loop_timing_receipt_arg());
    let baseline = full_command_run(0, true, 200);
    let row = super::super::timing::record::node_timing_row(
        crate::cli::live_loop::surfaces::surface_by_id("changed_files").expect("surface"),
        &command,
        "sha256:candidate",
        "sha256:changed",
        "sha256:audit",
        "sha256:input",
        &baseline,
        &verified_local_proof(0, true, 10, "missing_query_reconciliation"),
        "changed_files_digest_bound",
    );

    assert_eq!(row["timing_status"], "fail");
    assert_eq!(
        row["failure_class"],
        "live_loop_telemetry_reconciliation_missing"
    );
    assert_eq!(row["proof_kind"], "executed");
    assert_eq!(row["verified_local_command_argv"][0], "bash");
    assert!(
        row["output_digest"]
            .as_str()
            .expect("output digest")
            .starts_with("sha256:")
    );
    assert_eq!(row["output_digest"], row["verified_local_output_digest"]);
    assert!(
        row["verified_local_output_digest"]
            .as_str()
            .expect("output digest")
            .starts_with("sha256:")
    );
    assert_eq!(
        row["claim_impact"],
        "supports_live_loop_node_timing_only_no_readiness_release_completion_update_goal"
    );
}

#[test]
fn live_loop_measure_marks_executed_reconciled_speedup_as_pass() {
    let command = command(Some("changed_files"), live_loop_timing_receipt_arg());
    let baseline = full_command_run(0, true, 200);
    let row = super::super::timing::record::node_timing_row(
        crate::cli::live_loop::surfaces::surface_by_id("changed_files").expect("surface"),
        &command,
        "sha256:candidate",
        "sha256:changed",
        "sha256:audit",
        "sha256:input",
        &baseline,
        &verified_local_proof(0, true, 10, "pass"),
        "changed_files_digest_bound",
    );

    assert_eq!(row["timing_status"], "pass");
    assert_eq!(row["failure_class"], "none");
    assert_eq!(row["actual_work_duration_ms"], 10);
    assert_eq!(row["graph_overhead_ms"], 1);
}
