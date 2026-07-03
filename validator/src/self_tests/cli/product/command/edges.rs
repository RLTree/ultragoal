use std::path::PathBuf;

#[test]
fn product_command_reports_observability_receipt_write_failures() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("product-observability-write");
    std::fs::create_dir_all(root.join("target")).expect("target dir");
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        r#"{"resources":[]}"#,
    )
    .expect("manifest");
    std::fs::write(root.join("target/product-observability-blocker"), "blocker")
        .expect("blocker file");
    let err = crate::cli::product::run(
        &root,
        &crate::cli::product::ProductCommand {
            operation: crate::cli::product::ProductOperation::ProductProveCohesion,
            receipt_dir: None,
            observability_receipt: PathBuf::from(
                "target/product-observability-blocker/receipt.json",
            ),
        },
    )
    .expect_err("observability receipt write failure");
    assert!(err.contains("create parent failed"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup observability write");
}
