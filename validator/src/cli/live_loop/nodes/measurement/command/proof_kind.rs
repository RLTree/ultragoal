use super::fixtures::{
    command, full_command_run, live_loop_timing_receipt_arg, verified_local_proof,
};

#[test]
fn live_loop_measure_blocks_proof_shaped_timing_rows() {
    let command = command(Some("changed_files"), live_loop_timing_receipt_arg());
    let baseline = full_command_run(0, true, 200);
    let mut proof = verified_local_proof(0, true, 10, "pass");
    proof.proof_kind = "planned";
    let row = super::super::timing::record::node_timing_row(
        crate::cli::live_loop::surfaces::surface_by_id("changed_files").expect("surface"),
        &command,
        "sha256:candidate",
        "sha256:changed",
        "sha256:audit",
        "sha256:input",
        &baseline,
        &proof,
        "changed_files_digest_bound",
    );

    assert_eq!(row["timing_status"], "fail");
    assert_eq!(row["failure_class"], "verified_local_proof_kind_invalid");
    assert_eq!(row["where_failed"], "loop.measure.changed_files.proof_kind");
    assert!(
        row["why_failed"]
            .as_str()
            .expect("why")
            .contains("not executed or verified same-candidate cache")
    );
    assert!(
        row["next_repair"]
            .as_str()
            .expect("repair")
            .contains("replace proof-shaped timing")
    );
    assert_eq!(row["proof_surface"], "invalid proof_kind; row is blocked");
    assert_eq!(row["claim_status"], "blocked");
}
