use serde_json::json;

#[test]
fn authority_surface_inventory_discovers_nested_receipt_schemas() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "authority-receipt-schema-inventory",
    );
    let rel = "schemas/control/command-receipt.schema.json";
    std::fs::create_dir_all(root.join("schemas/control")).expect("schema dir");
    std::fs::write(root.join(rel), "{}").expect("schema file");
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({ "resources": [rel] })).expect("manifest json"),
    )
    .expect("manifest");

    let inventory = crate::audit::law::authority_surfaces::foundational_surface_inventory(&root);
    assert!(
        inventory
            .get("rows")
            .and_then(serde_json::Value::as_array)
            .expect("rows")
            .iter()
            .any(|row| row.get("role").and_then(serde_json::Value::as_str)
                == Some("receipt_schema")
                && row.get("path").and_then(serde_json::Value::as_str) == Some(rel)
                && row.get("surface_state").and_then(serde_json::Value::as_str)
                    == Some("available")),
        "{inventory}"
    );
    std::fs::remove_dir_all(root).expect("cleanup receipt schema inventory");
}
