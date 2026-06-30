use serde_json::json;

fn root(label: &str) -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::support::temp_root(label);
    std::fs::create_dir_all(root.join("validation_artifacts/improvement-loop"))
        .expect("improvement loop dir");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    )
    .expect("manifest");
    root
}

#[test]
fn improvement_loop_audit_rejects_missing_and_wrong_schema_receipts() {
    let root = root("audit-improvement-loop");
    let missing = super::package_failures(&root);
    assert!(
        missing
            .iter()
            .any(|failure| failure.starts_with("improvement_loop_receipt_missing_or_malformed")),
        "{missing:#?}"
    );

    crate::json_boundary::write_json(
        &root.join(super::RECEIPT_REL),
        &json!({"schema":"wrong","status":"pass"}),
    )
    .expect("wrong schema");
    let wrong_schema = super::package_failures(&root);
    assert!(
        wrong_schema
            .iter()
            .any(|failure| failure == "improvement_loop_receipt_wrong_schema"),
        "{wrong_schema:#?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}
