use serde_json::json;

fn root(label: &str) -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    std::fs::create_dir_all(root.join("validation_artifacts/promptfoo")).expect("promptfoo dir");
    crate::self_tests::boundaries::workspace_fixtures::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    )
    .expect("manifest");
    root
}

#[test]
fn promptfoo_audit_rejects_missing_and_wrong_schema_receipts() {
    let root = root("audit-promptfoo");
    let missing = super::package_failures(&root);
    assert!(
        missing.iter().any(|failure| {
            failure.starts_with("promptfoo_adapter_receipt_missing_or_malformed")
        }),
        "{missing:#?}"
    );

    crate::self_tests::boundaries::workspace_fixtures::write_json(
        &root.join(super::RECEIPT_REL),
        &json!({"schema":"wrong","status":"pass"}),
    )
    .expect("wrong schema");
    let wrong_schema = super::package_failures(&root);
    assert!(
        wrong_schema
            .iter()
            .any(|failure| failure == "promptfoo_adapter_receipt_wrong_schema"),
        "{wrong_schema:#?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}
