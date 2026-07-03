#[test]
fn raw_authority_projection_classification_is_closed_to_named_product_roles() {
    let product_projection = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/cli/observe/explain/summary.rs",
        "use serde_json::{Value, json};\nstruct ExplainContext<'a>{ observed: Option<&'a Value> }\npub fn explanation(ctx: ExplainContext<'_>) -> Value { json!({\"smallest_repair\":\"repair\",\"query_evidence\":ctx.observed}) }\n",
    );
    assert!(
        product_projection.is_empty(),
        "named product projection boundary should be classified: {product_projection:?}"
    );

    let generic_projection = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/cli/observe/explain/summary.rs",
        "use serde_json::{Value, json};\npub fn explanation(v: Value) -> Value { json!({\"status\":\"pass\",\"raw\":v}) }\n",
    );
    assert!(
        generic_projection
            .iter()
            .any(|failure| failure.contains("raw_downstream_authority_unclassified")),
        "projection path alone must not bless generic raw JSON passthrough: {generic_projection:?}"
    );
}
