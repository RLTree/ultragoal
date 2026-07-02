use super::*;
use std::fs;

#[test]
fn observe_snapshot_requires_selector_and_target_receipt() {
    let root = super::super::minimal_root("observe-snapshot-target-missing");
    let proof_rel = "validation_artifacts/observability/source-audit-production-proof.json";
    let no_selector = command(&["observe", "snapshot", "--receipt", proof_rel]);
    assert_eq!(observe::run(&root, &no_selector).expect("snapshot"), 1);
    let missing_selector = crate::json_boundary::read_json(&root.join(proof_rel)).expect("proof");
    assert!(
        missing_selector["why_failed"]
            .as_str()
            .unwrap()
            .contains("target_selector_missing"),
        "{missing_selector}"
    );

    let missing_target = command(&[
        "observe",
        "snapshot",
        "--run-id",
        "run-missing",
        "--receipt",
        proof_rel,
    ]);
    assert_eq!(observe::run(&root, &missing_target).expect("snapshot"), 1);
    let proof = crate::json_boundary::read_json(&root.join(proof_rel)).expect("proof");
    assert!(
        proof["why_failed"]
            .as_str()
            .unwrap()
            .contains("target_observability_receipt_missing:run_id=run-missing"),
        "{proof}"
    );
    assert_eq!(
        proof["next_repair"],
        "run the target command once on the current candidate, then rerun observe snapshot with the target run_id"
    );
    fs::remove_dir_all(root).expect("cleanup missing target");
}

#[test]
fn observe_snapshot_reports_missing_query_proof_before_fitting() {
    let root = super::super::minimal_root("observe-snapshot-missing-query-proof");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let target = write_source_audit_receipt(&root, &candidate);
    write_explain_receipt(&root, &candidate, &target, false);

    let proof_rel = "validation_artifacts/observability/source-audit-production-proof.json";
    let stale_command = command(&[
        "observe",
        "snapshot",
        "--run-id",
        &target.run_id,
        "--receipt",
        proof_rel,
    ]);
    assert_eq!(observe::run(&root, &stale_command).expect("snapshot"), 1);
    let proof = crate::json_boundary::read_json(&root.join(proof_rel)).expect("proof");
    assert!(
        proof["why_failed"]
            .as_str()
            .unwrap()
            .contains("same_candidate_logs_query_not_proven"),
        "{proof}"
    );
    assert_eq!(
        proof["next_repair"],
        "query logs metrics and traces for the target run/correlation/current digest, then rerun observe snapshot"
    );
    fs::remove_dir_all(root).expect("cleanup missing query proof");
}
