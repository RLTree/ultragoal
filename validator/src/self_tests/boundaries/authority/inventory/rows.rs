use serde_json::json;

#[test]
fn authority_surface_inventory_reports_required_product_roles() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("authority-surface-inventory");
    let required = crate::audit::law::authority_surfaces::required_surfaces_for_test();
    for (_, rel, _) in &required {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().expect("surface parent")).expect("surface dir");
        std::fs::write(&path, "{}").expect("surface file");
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
    assert_eq!(
        inventory["schema"],
        "harness-ultragoal.foundational-law-surface-inventory.v1"
    );
    assert_eq!(inventory["surface_count"], json!(required.len()));
    assert_eq!(inventory["missing_surface_count"], json!(0));
    assert_eq!(inventory["package_inventory_missing_count"], json!(0));
    for role in [
        "source",
        "schema",
        "law_registry",
        "research_registry",
        "research_trace",
        "valid_fixture",
        "red_fixture_catalog",
        "package_inventory",
        "generated_artifact",
        "receipt",
        "standards",
        "source_obligation",
        "foundational_trace",
        "claim_guard",
        "final_packet_blocker",
        "update_goal_blocker",
    ] {
        assert!(
            inventory.pointer(&format!("/role_counts/{role}")).is_some(),
            "missing role {role}: {inventory}"
        );
    }
    assert!(
        inventory
            .get("rows")
            .and_then(serde_json::Value::as_array)
            .expect("rows")
            .iter()
            .any(|row| row.get("path").and_then(serde_json::Value::as_str)
                == Some("validator/src/audit/law/authority_surfaces/surface_inventory.rs")
                && row.get("surface_state").and_then(serde_json::Value::as_str)
                    == Some("available")),
        "{inventory}"
    );
    assert!(
        inventory
            .get("rows")
            .and_then(serde_json::Value::as_array)
            .expect("rows")
            .iter()
            .any(|row| row.get("path").and_then(serde_json::Value::as_str)
                == Some("validator/src/audit/namespace/source/label_patterns.rs")
                && row.get("surface_state").and_then(serde_json::Value::as_str)
                    == Some("available")),
        "{inventory}"
    );
    std::fs::remove_dir_all(root).expect("cleanup authority surface inventory");
}

#[test]
fn authority_surface_inventory_marks_package_inventory_gaps() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "authority-surface-inventory-gap",
    );
    let required = crate::audit::law::authority_surfaces::required_surfaces_for_test();
    for (_, rel, _) in &required {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().expect("surface parent")).expect("surface dir");
        std::fs::write(&path, "{}").expect("surface file");
    }
    let omitted = "validator/src/audit/law/authority_surfaces/surface_inventory.rs";
    let resources = required
        .iter()
        .filter(|(_, rel, packaged)| *packaged && *rel != omitted)
        .map(|(_, rel, _)| *rel)
        .collect::<Vec<_>>();
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({ "resources": resources })).expect("manifest json"),
    )
    .expect("manifest");

    let inventory = crate::audit::law::authority_surfaces::foundational_surface_inventory(&root);
    assert_eq!(inventory["missing_surface_count"], json!(0));
    assert_eq!(inventory["package_inventory_missing_count"], json!(1));
    assert!(
        inventory
            .get("rows")
            .and_then(serde_json::Value::as_array)
            .expect("rows")
            .iter()
            .any(
                |row| row.get("path").and_then(serde_json::Value::as_str) == Some(omitted)
                    && row
                        .get("listed_in_package_inventory")
                        .and_then(serde_json::Value::as_bool)
                        == Some(false)
                    && row.get("surface_state").and_then(serde_json::Value::as_str)
                        == Some("blocked")
            ),
        "{inventory}"
    );
    std::fs::remove_dir_all(root).expect("cleanup authority surface inventory gap");
}
