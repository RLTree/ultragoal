use super::*;
use serde_json::json;
use std::fs;

#[test]
fn observe_snapshot_rejects_wrong_candidate_and_opaque_target_failures() {
    let root = super::super::minimal_root("observe-snapshot-target-edges");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let wrong_candidate = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    let target = write_source_audit_receipt(&root, wrong_candidate);
    write_query_receipts(&root, &candidate, &target);
    write_explain_receipt(&root, &candidate, &target, false);

    let proof_rel = "validation_artifacts/observability/source-audit-command-roundtrip.json";
    let opaque_command = command(&[
        "observe",
        "snapshot",
        "--run-id",
        &target.run_id,
        "--receipt",
        proof_rel,
    ]);
    assert_eq!(observe::run(&root, &opaque_command).expect("snapshot"), 1);
    let stale = crate::json_boundary::read_json(&root.join(proof_rel)).expect("proof");
    assert!(
        stale["why_failed"]
            .as_str()
            .unwrap()
            .contains("target_candidate_digest_mismatch"),
        "{stale}"
    );

    let opaque = write_opaque_target_receipt(&root, &candidate);
    write_query_receipts(&root, &candidate, &opaque);
    write_explain_receipt(&root, &candidate, &opaque, false);
    let command = command(&[
        "observe",
        "snapshot",
        "--run-id",
        &opaque.run_id,
        "--receipt",
        proof_rel,
    ]);
    assert_eq!(observe::run(&root, &command).expect("snapshot"), 1);
    let proof = crate::json_boundary::read_json(&root.join(proof_rel)).expect("proof");
    assert!(
        proof["why_failed"]
            .as_str()
            .unwrap()
            .contains("target_failure_is_not_agent_legible"),
        "{proof}"
    );
    fs::remove_dir_all(root).expect("cleanup target edges");
}

#[test]
fn observe_snapshot_rejects_receipts_without_event_binding() {
    let root = super::super::minimal_root("observe-snapshot-receipt-without-event");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let target = TargetIds {
        run_id: "run-without-event".to_string(),
        correlation_id: "corr-without-event".to_string(),
    };
    crate::json_boundary::write_json(
        &root.join("validation_artifacts/observability/source-audit.json"),
        &json!({
            "schema": RECEIPT_SCHEMA,
            "status": "fail",
            "candidate_digest": candidate,
            "operation": "source.audit",
            "run_id": target.run_id,
            "correlation_id": target.correlation_id
        }),
    )
    .expect("write eventless receipt");
    write_query_receipts(&root, &candidate, &target);
    write_explain_receipt(&root, &candidate, &target, false);

    let proof_rel = "validation_artifacts/observability/source-audit-command-roundtrip.json";
    let command = command(&[
        "observe",
        "snapshot",
        "--run-id",
        &target.run_id,
        "--receipt",
        proof_rel,
    ]);
    assert_eq!(observe::run(&root, &command).expect("snapshot"), 1);
    let proof = crate::json_boundary::read_json(&root.join(proof_rel)).expect("proof");
    assert!(
        proof["why_failed"]
            .as_str()
            .unwrap()
            .contains("target_receipt_without_event_binding"),
        "{proof}"
    );
    assert_eq!(
        proof["target_event"]["failure_class"],
        "receipt_without_observability_event"
    );
    fs::remove_dir_all(root).expect("cleanup eventless receipt");
}

#[test]
fn observe_snapshot_rejects_missing_candidate_and_nonmatching_receipts() {
    let root = super::super::minimal_root("observe-snapshot-candidate-missing");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let target = TargetIds {
        run_id: "run-missing-candidate".to_string(),
        correlation_id: "corr-missing-candidate".to_string(),
    };
    crate::json_boundary::write_json(
        &root.join("validation_artifacts/observability/source-audit.json"),
        &json!({
            "schema": RECEIPT_SCHEMA,
            "operation": "source.audit",
            "event": {
                "status": "fail",
                "operation": "source.audit",
                "run_id": target.run_id,
                "correlation_id": target.correlation_id,
                "failure_class": "source_audit_check_failure",
                "why_failed": "candidate digest is missing",
                "where_failed": "source.audit",
                "next_repair": "rerun source audit with candidate digest"
            }
        }),
    )
    .expect("write missing candidate target");
    write_query_receipts(&root, &candidate, &target);
    write_explain_receipt(&root, &candidate, &target, false);
    let proof_rel = "validation_artifacts/observability/source-audit-command-roundtrip.json";
    let missing_candidate = command(&[
        "observe",
        "snapshot",
        "--run-id",
        &target.run_id,
        "--receipt",
        proof_rel,
    ]);
    assert_eq!(
        observe::run(&root, &missing_candidate).expect("snapshot"),
        1
    );
    let proof = crate::json_boundary::read_json(&root.join(proof_rel)).expect("proof");
    assert!(
        proof["why_failed"]
            .as_str()
            .unwrap()
            .contains("target_candidate_digest_missing"),
        "{proof}"
    );

    let unmatched = command(&[
        "observe",
        "snapshot",
        "--run-id",
        "run-no-matching-target",
        "--receipt",
        proof_rel,
    ]);
    assert_eq!(observe::run(&root, &unmatched).expect("snapshot"), 1);
    let proof = crate::json_boundary::read_json(&root.join(proof_rel)).expect("proof");
    assert!(
        proof["why_failed"]
            .as_str()
            .unwrap()
            .contains("target_observability_receipt_missing:run_id=run-no-matching-target"),
        "{proof}"
    );
    fs::remove_dir_all(root).expect("cleanup missing candidate target");
}

fn write_opaque_target_receipt(root: &std::path::Path, candidate: &str) -> TargetIds {
    let target = TargetIds {
        run_id: "run-opaque-target".to_string(),
        correlation_id: "corr-opaque-target".to_string(),
    };
    crate::json_boundary::write_json(
        &root.join("validation_artifacts/observability/source-audit.json"),
        &json!({
            "schema": RECEIPT_SCHEMA,
            "operation": "source.audit",
            "event": {
                "schema": "harness-ultragoal.observability-event.v1",
                "status": "fail",
                "candidate_digest": candidate,
                "operation": "source.audit",
                "run_id": target.run_id,
                "correlation_id": target.correlation_id,
                "failure_class": "source_audit_check_failure",
                "where_failed": "source.audit",
                "next_repair": "repair opaque failure output"
            }
        }),
    )
    .expect("write opaque target");
    target
}
