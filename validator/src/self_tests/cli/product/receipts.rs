use std::path::PathBuf;

#[test]
fn product_receipt_minter_rebinds_canonical_source_local_receipts() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let rel = PathBuf::from(format!(
        "target/ultragoal-product-receipts-{}",
        std::process::id()
    ));
    let out = root.join(&rel);
    let _ = std::fs::remove_dir_all(&out);
    let candidate = crate::package::inventory::package_digest(&root).expect("digest before mint");

    let report = crate::cli::product::receipts::mint_all(&root, &rel, &out).expect("mint");
    assert_eq!(report["status"], "pass");
    assert!(report["failures"].as_array().expect("failures").is_empty());

    let fit = crate::json_boundary::read_json(&out.join("fit-repo-receipt.json")).expect("fit");
    let product =
        crate::json_boundary::read_json(&out.join("product-fitness-receipt.json")).expect("pf");
    let journey = crate::json_boundary::read_json(&out.join("plugin-product-journey-receipt.json"))
        .expect("journey");

    for value in [&fit, &product, &journey] {
        assert_eq!(value["target_revision"]["value"], candidate);
    }
    assert_eq!(
        fit["receipt_digest"],
        crate::audit::fit_repo_receipt::canonical_digest(&fit)
    );
    assert_eq!(
        product["receipt_digest"],
        crate::audit::product::fitness::receipt::canonical_digest(&product)
    );
    assert_eq!(
        journey["evidence"][0]["path"],
        format!("{}/fit-repo-receipt.json", rel.display())
    );

    std::fs::remove_dir_all(out).expect("cleanup");
}
