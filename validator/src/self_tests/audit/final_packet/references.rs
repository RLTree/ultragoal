use super::support;
use serde_json::{Value, json};
use std::path::Path;

#[test]
fn final_packet_proof_dereferences_receipts_instead_of_embedded_status() {
    let root = crate::self_tests::boundaries::support::temp_root("final-packet-proof-refs");
    let store = crate::schema_catalog::load(&crate::self_tests::boundaries::support::repo_root());
    support::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let current = crate::package::inventory::package_digest(&root).expect("digest");

    let mut missing_performance = support::write_green_proof(&root, &current);
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

    let mut bad_performance = support::write_green_proof(&root, &current);
    rewrite_ref(
        &root,
        &mut bad_performance,
        "cli_performance",
        "validation_artifacts/cli/performance-receipt.json",
        &json!({"status":"fail","digests":{"candidate":crate::self_tests::boundaries::support::sha('9')}}),
    );
    expect_failure(
        &root,
        &store,
        &bad_performance,
        "final_packet_proof_cli_performance_ref",
    );

    let mut bad_registry = support::write_green_proof(&root, &current);
    rewrite_ref(
        &root,
        &mut bad_registry,
        "registry_exposure",
        "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json",
        &json!({"status":"fail","target_revision":{"value":crate::self_tests::boundaries::support::sha('8')}}),
    );
    expect_failure(
        &root,
        &store,
        &bad_registry,
        "final_packet_proof_registry_ref",
    );

    let mut missing_registry = support::write_green_proof(&root, &current);
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

    let mut invalid_ref_path = support::write_green_proof(&root, &current);
    invalid_ref_path["cli_performance"]["path"] = json!("../performance-receipt.json");
    expect_failure(
        &root,
        &store,
        &invalid_ref_path,
        "final_packet_proof_ref_path_invalid:cli_performance",
    );

    let mut malformed_ref = support::write_green_proof(&root, &current);
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

    let mut bad_source_audit = support::write_green_proof(&root, &current);
    rewrite_ref(
        &root,
        &mut bad_source_audit,
        "source_audit",
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
        &json!({"status":"fail","target_revision":{"value":crate::self_tests::boundaries::support::sha('7')}}),
    );
    expect_failure(
        &root,
        &store,
        &bad_source_audit,
        "final_packet_proof_source_audit_target_digest_mismatch",
    );

    let mut honest_failed_source_audit = support::write_green_proof(&root, &current);
    rewrite_ref(
        &root,
        &mut honest_failed_source_audit,
        "source_audit",
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
        &json!({"status":"fail","target_revision":{"value":current}}),
    );
    honest_failed_source_audit["source_audit"]["status"] = json!("fail");
    support::write_proof(&root, &honest_failed_source_audit);
    let failures = crate::audit::final_packet::package_failures(&root, &store);
    assert!(failures.is_empty(), "{failures:?}");

    let mut bad_embedded_source_status = support::write_green_proof(&root, &current);
    bad_embedded_source_status["source_audit"]["status"] = json!("pending");
    expect_failure(
        &root,
        &store,
        &bad_embedded_source_status,
        "final_packet_proof_ref_embedded_status_not_pass_or_fail:source_audit",
    );

    let mut missing_source_status = support::write_green_proof(&root, &current);
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

    let mut unknown_source_status = support::write_green_proof(&root, &current);
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

    let mut bad_coverage = support::write_green_proof(&root, &current);
    rewrite_ref(
        &root,
        &mut bad_coverage,
        "coverage",
        "validation_artifacts/coverage/coverage-receipt.json",
        &json!({"target_revision":{"value":crate::self_tests::boundaries::support::sha('6')},
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

    let mut missing_packages = support::write_green_proof(&root, &current);
    missing_packages
        .as_object_mut()
        .expect("proof object")
        .remove("package_receipts");
    expect_failure(
        &root,
        &store,
        &missing_packages,
        "final_packet_proof_package_receipts_missing",
    );

    let mut bad_package = support::write_green_proof(&root, &current);
    rewrite_ref(
        &root,
        &mut bad_package,
        "package_receipts/0",
        "validation_artifacts/harness/package-receipt.json",
        &json!({"status":"pass","target_revision":{"value":crate::self_tests::boundaries::support::sha('5')}}),
    );
    expect_failure(
        &root,
        &store,
        &bad_package,
        "final_packet_proof_package_target_digest_mismatch",
    );

    let mut bad_fit_repo = support::write_green_proof(&root, &current);
    rewrite_ref(
        &root,
        &mut bad_fit_repo,
        "package_receipts/0",
        "validation_artifacts/harness/fit-repo-receipt.json",
        &json!({"status":"pass","target_revision":{"value":current}}),
    );
    expect_failure(
        &root,
        &store,
        &bad_fit_repo,
        "final_packet_proof_fit_repo_ref",
    );
    std::fs::remove_dir_all(root).expect("cleanup final packet proof references");
}

fn rewrite_ref(root: &Path, proof: &mut Value, key: &str, path: &str, value: &Value) {
    support::write_json(&root.join(path), value);
    let item = proof
        .pointer_mut(&format!("/{key}"))
        .expect("proof reference");
    item["path"] = json!(path);
    item["digest"] = json!(crate::digest::file(&root.join(path)).expect("reference digest"));
}

fn expect_failure(
    root: &Path,
    store: &crate::schema_catalog::SchemaStore,
    proof: &Value,
    expected: &str,
) {
    support::write_proof(root, proof);
    let failures = crate::audit::final_packet::package_failures(root, store);
    assert!(
        failures.iter().any(|failure| failure.contains(expected)),
        "{expected}: {failures:?}"
    );
}
