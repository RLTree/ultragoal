use serde_json::json;

#[test]
fn schema_dispatch_reaches_product_cohesion_rules() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let store = crate::schema_catalog::load(&root);
    let errors = crate::schema_catalog::schema_errors(
        &store,
        "product-cohesion-receipt.schema.json",
        &json!({}),
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("product_surface_id is required")),
        "{errors:?}"
    );
}
