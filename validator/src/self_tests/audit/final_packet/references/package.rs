use super::{expect_failure, receipt_fixtures, rewrite_ref};
use serde_json::json;

#[test]
fn final_packet_package_receipts_are_dereferenced_by_concrete_receipt_type() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("final-packet-package-refs");
    let store = crate::schema_catalog::load(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root(),
    );
    receipt_fixtures::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let current = crate::package::inventory::package_digest(&root).expect("digest");

    let mut missing_packages = receipt_fixtures::write_green_proof(&root, &current);
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

    let mut missing_product_fitness = receipt_fixtures::write_green_proof(&root, &current);
    missing_product_fitness["package_receipts"]
        .as_array_mut()
        .expect("package refs")
        .remove(1);
    expect_failure(
        &root,
        &store,
        &missing_product_fitness,
        "final_packet_proof_package_receipt_missing:validation_artifacts/harness/product-fitness-receipt.json",
    );

    let mut bad_package = receipt_fixtures::write_green_proof(&root, &current);
    rewrite_ref(
        &root,
        &mut bad_package,
        "package_receipts/0",
        "validation_artifacts/harness/package-receipt.json",
        &json!({"status":"pass","target_revision":{"value":crate::self_tests::boundaries::workspace_fixtures::sha('5')}}),
    );
    expect_failure(
        &root,
        &store,
        &bad_package,
        "final_packet_proof_package_target_digest_mismatch",
    );

    let mut bad_fit_repo = receipt_fixtures::write_green_proof(&root, &current);
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

    let mut bad_product_fitness = receipt_fixtures::write_green_proof(&root, &current);
    rewrite_ref(
        &root,
        &mut bad_product_fitness,
        "package_receipts/1",
        "validation_artifacts/harness/product-fitness-receipt.json",
        &json!({"schema":"harness-ultragoal.product-fitness-receipt.v1",
            "claim":{"id":"WRONG"},
            "target_revision":{"kind":"package_digest","value":current},
            "receipt_digest":crate::digest::ZERO}),
    );
    expect_failure(
        &root,
        &store,
        &bad_product_fitness,
        "final_packet_proof_product_fitness_ref",
    );

    let mut bad_product_journey = receipt_fixtures::write_green_proof(&root, &current);
    rewrite_ref(
        &root,
        &mut bad_product_journey,
        "package_receipts/2",
        "validation_artifacts/harness/plugin-product-journey-receipt.json",
        &json!({"schema":"harness-ultragoal.plugin-product-journey-receipt.v1",
            "status":"fail",
            "target_revision":{"kind":"package_digest","value":current},
            "journey":[]}),
    );
    expect_failure(
        &root,
        &store,
        &bad_product_journey,
        "final_packet_proof_product_journey_ref",
    );

    std::fs::remove_dir_all(root).expect("cleanup final packet package refs");
}
