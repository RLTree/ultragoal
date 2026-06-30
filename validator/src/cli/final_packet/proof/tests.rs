use serde_json::{Value, json};
use std::path::PathBuf;

fn root(label: &str) -> PathBuf {
    let root = crate::self_tests::boundaries::support::temp_root(label);
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    )
    .expect("manifest");
    root
}

fn failing(failure: &str) -> Value {
    json!({
        "status": "fail",
        "blocked_claim_classes": ["completion"],
        "failure": {"observed_failures": [failure]}
    })
}

#[test]
fn final_packet_observability_classifies_repair_paths() {
    let root = root("final-packet-proof-repair-paths");
    let receipt = root.join("validation_artifacts/review/final-packet-proof.json");

    let mut source_audit = failing("final_packet_proof_source_audit_target_digest_mismatch");
    super::attach_for_evaluation(&root, &receipt, &mut source_audit).expect("source attach");
    assert_eq!(source_audit["proof_check_id"], "wrong_digest");
    assert!(
        source_audit["next_repair"]
            .as_str()
            .unwrap_or_default()
            .contains("source audit receipt")
    );

    let mut coverage = failing("final_packet_proof_coverage_target_digest_mismatch");
    super::attach_for_evaluation(&root, &receipt, &mut coverage).expect("coverage attach");
    assert_eq!(coverage["proof_check_id"], "wrong_digest");
    assert!(
        coverage["next_repair"]
            .as_str()
            .unwrap_or_default()
            .contains("exact coverage")
    );

    let mut packet_absent = failing("final_packet_proof_packet_absent");
    super::attach_for_evaluation(&root, &receipt, &mut packet_absent).expect("packet attach");
    assert_eq!(packet_absent["proof_check_id"], "missing_receipt");
    assert!(
        packet_absent["next_repair"]
            .as_str()
            .unwrap_or_default()
            .contains("source-local proof graph")
    );

    let mut registry = failing("final_packet_proof_registry_ref:unsupported");
    super::attach_for_evaluation(&root, &receipt, &mut registry).expect("registry attach");
    assert_eq!(registry["proof_check_id"], "unsupported_live_surface");

    let mut generic = failing("final_packet_proof_claim_guard_failed");
    super::attach_for_evaluation(&root, &receipt, &mut generic).expect("generic attach");
    assert_eq!(generic["proof_check_id"], "claim_blocked");

    let mut pass = json!({"status":"pass","blocked_claim_classes":[]});
    super::attach_for_evaluation(&root, &receipt, &mut pass).expect("pass attach");
    assert_eq!(pass["why_failed"], "none");
    assert_eq!(pass["where_failed"], "none");
    assert_eq!(
        pass["observability"]["supported_claims"][0],
        "final_packet_evidence_dereferenced"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn final_packet_observability_fails_when_package_digest_is_unavailable() {
    let root = crate::self_tests::boundaries::support::temp_root("final-packet-no-manifest");
    let receipt = root.join("validation_artifacts/review/final-packet-proof.json");
    let mut proof = failing("final_packet_proof_packet_absent");
    let err = super::attach_for_evaluation(&root, &receipt, &mut proof)
        .expect_err("missing manifest blocks telemetry binding");
    assert!(err.contains("plugin-manifest-draft.json"));
}
