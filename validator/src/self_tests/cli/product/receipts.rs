use std::path::PathBuf;

#[test]
fn product_receipt_minter_rebinds_canonical_source_local_receipts() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let rel = PathBuf::from(format!(
        "target/ultragoal-product-receipts-{}",
        std::process::id()
    ));
    let receipt_dir = root.join(&rel);
    let _ = std::fs::remove_dir_all(&receipt_dir);
    let candidate = crate::package::inventory::package_digest(&root).expect("digest before mint");

    let report = crate::cli::product::receipts::mint_all(&root, &rel).expect("mint");
    assert_eq!(report["status"], "pass");
    assert!(report["failures"].as_array().expect("failures").is_empty());

    let fit_repo_receipt =
        crate::json_boundary::read_json(&receipt_dir.join("fit-repo-receipt.json"))
            .expect("fit-repo receipt");
    let product =
        crate::json_boundary::read_json(&receipt_dir.join("product-fitness-receipt.json"))
            .expect("pf");
    let journey =
        crate::json_boundary::read_json(&receipt_dir.join("plugin-product-journey-receipt.json"))
            .expect("journey");

    for value in [&fit_repo_receipt, &product, &journey] {
        assert_eq!(value["target_revision"]["value"], candidate);
    }
    assert_eq!(
        fit_repo_receipt["receipt_digest"],
        crate::audit::fit_repo_receipt::canonical_digest(&fit_repo_receipt)
    );
    assert_eq!(
        product["receipt_digest"],
        crate::audit::product::fitness::receipt::canonical_digest(&product)
    );
    assert_eq!(
        journey["evidence"][0]["path"],
        format!("{}/fit-repo-receipt.json", rel.display())
    );

    std::fs::remove_dir_all(receipt_dir).expect("cleanup");
}
