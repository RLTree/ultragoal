use super::fixtures::{
    command, full_command_run, live_loop_timing_receipt_arg, verified_local_proof,
};

#[test]
fn live_loop_measure_projects_empty_affected_set_and_failure_classes() {
    assert_eq!(
        super::super::timing::record::timing_status("pass", "supported"),
        "pass"
    );
    assert_eq!(
        super::super::timing::record::timing_status("pass", "withheld"),
        "partial"
    );
    assert_eq!(
        super::super::timing::record::timing_status("fail", "withheld"),
        "fail"
    );
    assert_eq!(
        super::super::timing::receipt::affected_set_status(0),
        "clean_worktree_no_affected_files"
    );
    assert_eq!(
        super::super::timing::receipt::affected_set_status(1),
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

    assert_eq!(row["timing_status"], "partial");
    assert_eq!(row["validation_status"], "pass");
    assert_eq!(row["validation_cache_status"], "reusable");
    assert_eq!(row["observability_status"], "partial");
    assert_eq!(row["speed_claim_status"], "withheld");
    assert_eq!(
        row["observability_failure_class"],
        "live_loop_observability_partial"
    );
    assert_eq!(
        row["failure_class"],
        "live_loop_telemetry_reconciliation_missing"
    );
    assert_eq!(row["proof_kind"], "executed");
    assert_eq!(row["baseline_proof_kind"], "executed_same_command_reuse");
    assert_eq!(row["telemetry_reconciliation_duration_ms"], 1);
    assert_eq!(row["reconciled_command_duration_ms"], 12);
    assert_eq!(row["product_latency_ms"], 11);
    assert_eq!(
        row["baseline_invalidation_proof"],
        "baseline_reused_from_executed_narrow_command_because_canonical_full_command_matches"
    );
    assert_eq!(row["command_argv"][0], "git");
    assert_eq!(row["exit_status"], 0);
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
    assert_eq!(
        row["claim_ceiling"],
        "source-local loop timing only; readiness release completion final-packet and update_goal remain blocked"
    );
}
