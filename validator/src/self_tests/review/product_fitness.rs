#[test]
fn product_fitness_and_target_fixture_boundaries_fail_closed() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("product-fitness");
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::create_dir_all(root.join("templates")).expect("templates");
    std::fs::create_dir_all(root.join("schemas")).expect("schemas");
    std::fs::create_dir_all(root.join("validation_artifacts/harness")).expect("harness");
    std::fs::write(
        root.join("docs/product-fitness-and-quality-in-use.md"),
        "This should be strict.",
    )
    .expect("law");
    std::fs::write(
        root.join("templates/PRODUCT_FITNESS.md"),
        "This may be strict.",
    )
    .expect("template");
    std::fs::write(
        root.join("schemas/product-fitness-receipt.schema.json"),
        "{}",
    )
    .expect("schema");
    std::fs::write(
        root.join("validation_artifacts/harness/product-fitness-receipt.json"),
        "{}",
    )
    .expect("receipt");
    let failures = crate::audit::product::fitness::package_failures(&root);
    assert!(
        failures
            .iter()
            .any(|item| item.starts_with("product_fitness_discretionary_language"))
    );
    assert!(
        failures
            .iter()
            .any(|item| item == "product_fitness_receipt_wrong_claim_id")
    );
    assert!(
        failures
            .iter()
            .any(|item| item == "product_fitness_receipt_malformed:schema")
    );
    assert!(
        failures
            .iter()
            .any(|item| item == "product_fitness_receipt_stale")
    );

    let missing = crate::target_fixtures::target_capability_failures(&root, &[]);
    assert!(
        missing
            .iter()
            .any(|item| item.starts_with("missing target-repo audit files"))
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}
