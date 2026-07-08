use serde_json::json;

#[test]
fn missing_runtime_materializations_do_not_increment_source_surface_missing_count() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "authority-runtime-materialization-count",
    );
    let required = crate::audit::law::authority_surfaces::required_surfaces_for_test();
    for (_, rel, packaged) in &required {
        if *packaged {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().expect("surface parent")).expect("surface dir");
            std::fs::write(&path, "{}").expect("surface file");
        }
    }
    let resources = required
        .iter()
        .filter(|(_, _, packaged)| *packaged)
        .map(|(_, rel, _)| *rel)
        .collect::<Vec<_>>();
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({ "resources": resources })).expect("manifest json"),
    )
    .expect("manifest");

    let inventory = crate::audit::law::authority_surfaces::foundational_surface_inventory(&root);
    assert_eq!(inventory["missing_surface_count"], json!(0), "{inventory}");
    assert!(
        inventory
            .get("rows")
            .and_then(serde_json::Value::as_array)
            .expect("rows")
            .iter()
            .any(|row| row.get("path").and_then(serde_json::Value::as_str)
                == Some("validation_artifacts/coverage/coverage-receipt.json")
                && row
                    .get("exists_on_disk")
                    .and_then(serde_json::Value::as_bool)
                    == Some(false)
                && row
                    .get("existence_required")
                    .and_then(serde_json::Value::as_bool)
                    == Some(false)
                && row.get("surface_state").and_then(serde_json::Value::as_str)
                    == Some("available")),
        "{inventory}"
    );
    std::fs::remove_dir_all(root).expect("cleanup runtime materialization count");
}
