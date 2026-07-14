#[test]
fn package_artifact_exposes_captured_source_identity_fields() {
    let repo = Repo::new("package-artifact-identity-fields");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let artifact = capture_product_package(&context, &authority_catalog).unwrap();

    assert_eq!(artifact.context_id(), artifact.snapshot().context_id());
    assert_eq!(artifact.candidate_id(), artifact.snapshot().candidate_id());
    assert_eq!(artifact.catalog_id(), artifact.snapshot().catalog_id());
    assert!(artifact.source_snapshot_id().starts_with("sha256:"));
    let source_inventory = std::str::from_utf8(artifact.source_inventory()).unwrap();
    assert!(source_inventory.contains("harness-ultragoal.accepted-package-source-set.v1"));
}
