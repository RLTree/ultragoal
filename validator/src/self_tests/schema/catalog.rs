use serde_json::json;
use std::collections::BTreeSet;

#[test]
fn catalog_refs_report_duplicates_missing_extra_and_offline_ref_failures() {
    let root = crate::self_tests::boundaries::support::temp_root("schema-catalog-refs");
    std::fs::create_dir_all(root.join("schemas")).expect("schemas dir");
    std::fs::write(root.join("schemas/present.schema.json"), "{}").expect("schema");
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({"schemas":["schemas/manifest.schema.json"]})).expect("json"),
    )
    .expect("manifest");

    let rows = vec![
        json!({"id":"dup","path":"schemas/extra.schema.json"}),
        json!({"id":"dup","path":"schemas/extra.schema.json"}),
    ];
    let errors = crate::schema_catalog::catalog_refs::catalog_completeness_errors(&root, &rows);
    assert!(errors.iter().any(|error| error.contains("duplicate paths")));
    assert!(errors.iter().any(|error| error.contains("duplicate ids")));
    assert!(
        errors
            .iter()
            .any(|error| error.contains("missing schema paths"))
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("extra schema paths"))
    );

    let mut allowed = BTreeSet::new();
    allowed.insert("schemas/known.schema.json".to_string());
    let ref_errors = crate::schema_catalog::catalog_refs::ref_errors(
        &json!({
            "allOf":[
                {"$ref":"https://example.invalid/schema.json"},
                {"items":{"$ref":"schemas/missing.schema.json#/defs/x"}},
                {"$ref":"schemas/known.schema.json#/defs/y"}
            ]
        }),
        &allowed,
    );
    assert!(
        ref_errors
            .iter()
            .any(|error| error.contains("not in offline catalog"))
    );
    assert!(
        ref_errors
            .iter()
            .any(|error| error.contains("missing from offline catalog"))
    );
    std::fs::remove_dir_all(root).expect("cleanup schema catalog refs");
}

#[test]
fn catalog_refs_accept_empty_schema_surface_when_manifest_unreadable() {
    let root = crate::self_tests::boundaries::support::temp_root("schema-catalog-empty");
    std::fs::create_dir_all(&root).expect("root");
    let errors = crate::schema_catalog::catalog_refs::catalog_completeness_errors(&root, &[]);
    assert!(errors.is_empty(), "{errors:?}");
    std::fs::remove_dir_all(root).expect("cleanup empty schema catalog");
}
