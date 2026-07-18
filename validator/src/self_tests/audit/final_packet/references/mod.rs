use super::receipt_fixtures;
use serde_json::{Value, json};
use std::path::Path;

mod package;

#[test]
fn final_packet_proof_dereferences_receipts_instead_of_embedded_status() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("final-packet-proof-refs");
    let store = crate::schema_catalog::load(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root(),
    );
    receipt_fixtures::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let current = crate::package::inventory::package_digest(&root).expect("digest");

    let mut missing_performance = receipt_fixtures::write_green_proof(&root, &current);
    missing_performance
        .as_object_mut()
        .expect("proof object")
        .remove("cli_performance");
    expect_failure(
        &root,
        &store,
        &missing_performance,
        "final_packet_proof_ref_missing:cli_performance",
    );

    let mut bad_performance = receipt_fixtures::write_green_proof(&root, &current);
    rewrite_ref(
        &root,
        &mut bad_performance,
        "cli_performance",
        "validation_artifacts/cli/performance-receipt.json",
        &json!({"status":"fail","digests":{"candidate":crate::self_tests::boundaries::workspace_fixtures::sha('9')}}),
    );
    expect_failure(
        &root,
        &store,
        &bad_performance,
        "final_packet_proof_cli_performance_ref",
    );

    let mut bad_registry = receipt_fixtures::write_green_proof(&root, &current);
    rewrite_ref(
        &root,
        &mut bad_registry,
        "registry_exposure",
        "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json",
        &json!({"status":"fail","target_revision":{"value":crate::self_tests::boundaries::workspace_fixtures::sha('8')}}),
    );
    expect_failure(
        &root,
        &store,
        &bad_registry,
        "final_packet_proof_registry_ref",
    );

    let mut missing_registry = receipt_fixtures::write_green_proof(&root, &current);
    missing_registry
        .as_object_mut()
        .expect("proof object")
        .remove("registry_exposure");
    expect_failure(
        &root,
        &store,
        &missing_registry,
        "final_packet_proof_ref_missing:registry",
    );

    let mut invalid_ref_path = receipt_fixtures::write_green_proof(&root, &current);
    invalid_ref_path["cli_performance"]["path"] = json!("../performance-receipt.json");
    expect_failure(
        &root,
        &store,
        &invalid_ref_path,
        "final_packet_proof_ref_path_invalid:cli_performance",
    );

    let mut misplaced_self_rewrite = receipt_fixtures::write_green_proof(&root, &current);
    misplaced_self_rewrite["cli_performance"]["self_rewriting_authority"] =
        json!("source_audit_command_writes_validator_receipt");
    expect_failure(
        &root,
        &store,
        &misplaced_self_rewrite,
        "final_packet_proof_ref_unexpected_self_rewrite_authority:cli_performance",
    );

    let mut malformed_ref = receipt_fixtures::write_green_proof(&root, &current);
    let malformed_path = "validation_artifacts/cli/performance-receipt.json";
    std::fs::write(root.join(malformed_path), "{").expect("write malformed reference");
    malformed_ref["cli_performance"]["digest"] =
        json!(crate::digest::file(&root.join(malformed_path)).expect("malformed digest"));
    expect_failure(
        &root,
        &store,
        &malformed_ref,
        "final_packet_proof_ref_malformed:cli_performance",
    );

    let mut bad_source_audit = receipt_fixtures::write_green_proof(&root, &current);
    rewrite_ref(
        &root,
        &mut bad_source_audit,
        "source_audit",
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
        &json!({"status":"fail","target_revision":{"value":crate::self_tests::boundaries::workspace_fixtures::sha('7')}}),
    );
    expect_failure(
        &root,
        &store,
        &bad_source_audit,
        "final_packet_proof_source_audit_target_digest_mismatch",
    );

    let mut honest_failed_source_audit = receipt_fixtures::write_green_proof(&root, &current);
    rewrite_ref(
        &root,
        &mut honest_failed_source_audit,
        "source_audit",
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
        &json!({"status":"fail","target_revision":{"value":current}}),
    );
    honest_failed_source_audit["source_audit"]["status"] = json!("fail");
    expect_failure(
        &root,
        &store,
        &honest_failed_source_audit,
        "final_packet_proof_source_audit_status_not_pass",
    );

    let mut bad_embedded_source_status = receipt_fixtures::write_green_proof(&root, &current);
    bad_embedded_source_status["source_audit"]["status"] = json!("pending");
    expect_failure(
        &root,
        &store,
        &bad_embedded_source_status,
        "final_packet_proof_ref_embedded_status_not_pass:source_audit",
    );

    let mut missing_source_status = receipt_fixtures::write_green_proof(&root, &current);
    rewrite_ref(
        &root,
        &mut missing_source_status,
        "source_audit",
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
        &json!({"target_revision":{"value":current}}),
    );
    expect_failure(
        &root,
        &store,
        &missing_source_status,
        "final_packet_proof_source_audit_status_missing",
    );

    let mut unknown_source_status = receipt_fixtures::write_green_proof(&root, &current);
    rewrite_ref(
        &root,
        &mut unknown_source_status,
        "source_audit",
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
        &json!({"status":"pending","target_revision":{"value":current}}),
    );
    expect_failure(
        &root,
        &store,
        &unknown_source_status,
        "final_packet_proof_source_audit_status_unknown:pending",
    );

    let mut bad_coverage = receipt_fixtures::write_green_proof(&root, &current);
    rewrite_ref(
        &root,
        &mut bad_coverage,
        "coverage",
        "validation_artifacts/coverage/coverage-receipt.json",
        &json!({"target_revision":{"value":crate::self_tests::boundaries::workspace_fixtures::sha('6')},
            "coverage":{"percent":99.0},"uncovered_records":[{"path":"src/lib.rs"}],
            "claim_ceiling":"withheld_or_blocked"}),
    );
    for expected in [
        "final_packet_proof_coverage_target_digest_mismatch",
        "final_packet_proof_coverage_not_exact_100",
        "final_packet_proof_coverage_claim_ceiling_not_complete",
    ] {
        expect_failure(&root, &store, &bad_coverage, expected);
    }

    std::fs::remove_dir_all(root).expect("cleanup final packet proof references");
}

pub(super) fn rewrite_ref(root: &Path, proof: &mut Value, key: &str, path: &str, value: &Value) {
    receipt_fixtures::write_json(&root.join(path), value);
    let item = proof
        .pointer_mut(&format!("/{key}"))
        .expect("proof reference");
    item["path"] = json!(path);
    item["digest"] = json!(crate::digest::file(&root.join(path)).expect("reference digest"));
}

pub(super) fn expect_failure(
    root: &Path,
    store: &crate::schema_catalog::SchemaStore,
    proof: &Value,
    expected: &str,
) {
    receipt_fixtures::write_proof(root, proof);
    let failures = crate::audit::final_packet::value_failures(root, store, proof);
    assert!(
        failures.iter().any(|failure| failure.contains(expected)),
        "{expected}: {failures:?}"
    );
}
