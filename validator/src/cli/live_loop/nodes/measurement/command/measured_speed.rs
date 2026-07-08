use super::fixtures::{
    command, full_command_run, live_loop_timing_receipt_arg, verified_local_proof,
};

#[test]
fn live_loop_measure_marks_executed_reconciled_speedup_as_pass() {
    let command = command(Some("fmt_check"), live_loop_timing_receipt_arg());
    let baseline = full_command_run(0, true, 240);
    let row = super::super::timing::record::node_timing_row(
        crate::cli::live_loop::surfaces::surface_by_id("fmt_check").expect("surface"),
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
    assert_eq!(row["telemetry_reconciliation_duration_ms"], 1);
    assert_eq!(row["reconciled_command_duration_ms"], 12);
    assert_eq!(row["product_latency_ms"], 11);
    assert_eq!(
        row["claim_ceiling"],
        "source-local loop timing only; readiness release completion final-packet and update_goal remain blocked"
    );
}
