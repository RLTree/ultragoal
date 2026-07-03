use serde_json::json;

mod claim_guard;
mod references;
mod registry;
mod source_audit;
pub(crate) mod support;

#[test]
fn final_packet_proof_requires_same_candidate_dereferenced_packet() {
    let root = crate::self_tests::boundaries::support::temp_root("final-packet-proof");
    let store = crate::schema_catalog::load(&crate::self_tests::boundaries::support::repo_root());
    let missing = crate::audit::final_packet::package_failures(&root, &store);
    assert!(
        missing
            .iter()
            .any(|failure| failure.contains("final_packet_proof_missing")),
        "{missing:?}"
    );

    support::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let current = crate::package::inventory::package_digest(&root).expect("digest");
    let receipt = support::write_green_proof(&root, &current);
    let failures = crate::audit::final_packet::package_failures(&root, &store);
    assert!(failures.is_empty(), "{failures:?}");

    let mut missing_observability = receipt.clone();
    missing_observability
        .as_object_mut()
        .expect("receipt object")
        .remove("observability");
    support::write_proof(&root, &missing_observability);
    let failures = crate::audit::final_packet::package_failures(&root, &store);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("final_packet_proof_observability_missing")),
        "{failures:?}"
    );

    let mut corrupt_observability = receipt.clone();
    corrupt_observability["observability"]["event"]["candidate_digest"] =
        json!(crate::self_tests::boundaries::support::sha('c'));
    support::write_proof(&root, &corrupt_observability);
    let failures = crate::audit::final_packet::package_failures(&root, &store);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("final_packet_proof_observability_digest_mismatch")),
        "{failures:?}"
    );

    let mut bad_status = receipt.clone();
    bad_status["status"] = json!("fail");
    bad_status["claim_ceiling"] = json!("withheld_or_blocked");
    bad_status["cli_performance"]["status"] = json!("fail");
    support::write_proof(&root, &bad_status);
    let failures = crate::audit::final_packet::package_failures(&root, &store);
    for expected in [
        "final_packet_authority_status_not_pass",
        "final_packet_proof_claim_ceiling_not_verified",
        "final_packet_proof_cli_performance_not_pass",
    ] {
        assert!(
            failures.iter().any(|failure| failure.contains(expected)),
            "{expected}: {failures:?}"
        );
    }

    let mut disagree = receipt.clone();
    let mut referenced = crate::json_boundary::read_json(
        &root.join("validation_artifacts/cli/performance-receipt.json"),
    )
    .expect("performance ref");
    referenced["status"] = json!("fail");
    let cli_path = "validation_artifacts/cli/performance-receipt.json";
    support::write_json(&root.join(cli_path), &referenced);
    disagree["cli_performance"]["digest"] =
        json!(crate::digest::file(&root.join(cli_path)).expect("changed cli digest"));
    support::write_proof(&root, &disagree);
    let failures = crate::audit::final_packet::package_failures(&root, &store);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("final_packet_proof_ref_status_disagreement")),
        "{failures:?}"
    );

    let mut invalid_path = receipt.clone();
    invalid_path["packet"]["path"] = json!("../final-packet.json");
    support::write_proof(&root, &invalid_path);
    let failures = crate::audit::final_packet::package_failures(&root, &store);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("final_packet_proof_packet_path_invalid")),
        "{failures:?}"
    );

    let mut bad_digest = receipt.clone();
    bad_digest["packet"]["digest"] = json!(crate::self_tests::boundaries::support::sha('e'));
    support::write_proof(&root, &bad_digest);
    let failures = crate::audit::final_packet::package_failures(&root, &store);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("final_packet_proof_packet_digest_mismatch")),
        "{failures:?}"
    );

    let mut zero_digest = receipt.clone();
    zero_digest["packet"]["digest"] = json!(crate::digest::ZERO);
    support::write_proof(&root, &zero_digest);
    let failures = crate::audit::final_packet::package_failures(&root, &store);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("final_packet_proof_packet_zero_digest_anchor")),
        "{failures:?}"
    );

    let mut missing_packet = receipt.clone();
    missing_packet["packet"]["path"] = json!("validation_artifacts/review/missing-packet.json");
    support::write_proof(&root, &missing_packet);
    let failures = crate::audit::final_packet::package_failures(&root, &store);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("final_packet_proof_packet_digest_mismatch")),
        "{failures:?}"
    );

    let mut absent_bad_digest = receipt.clone();
    absent_bad_digest["packet"]["exists"] = json!(false);
    absent_bad_digest["packet"]["digest"] = json!(crate::self_tests::boundaries::support::sha('a'));
    support::write_proof(&root, &absent_bad_digest);
    let failures = crate::audit::final_packet::package_failures(&root, &store);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("final_packet_proof_packet_absent_digest_not_null")),
        "{failures:?}"
    );

    let mut stale = receipt;
    stale["target_revision"]["value"] = json!(crate::self_tests::boundaries::support::sha('d'));
    support::write_proof(&root, &stale);
    let failures = crate::audit::final_packet::package_failures(&root, &store);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("final_packet_proof_target_digest_mismatch")),
        "{failures:?}"
    );

    std::fs::remove_dir_all(root).expect("cleanup final packet proof");
}
