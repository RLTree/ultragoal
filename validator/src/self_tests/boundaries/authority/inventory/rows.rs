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
    assert!(
        inventory["surface_count"].as_u64().unwrap_or_default() >= required.len() as u64,
        "{inventory}"
    );
    assert_eq!(inventory["missing_surface_count"], json!(0));
    assert_eq!(inventory["package_inventory_missing_count"], json!(0));
    for role in [
        "source",
        "schema",
        "receipt_schema",
        "law_registry",
        "research_registry",
        "research_trace",
        "valid_fixture",
        "red_fixture_catalog",
        "package_manifest",
        "package_inventory",
        "setup_retrofit_output",
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
                == Some("validator/src/audit/law/authority_surfaces/surface_inventory/mod.rs")
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
    let omitted = "validator/src/audit/law/authority_surfaces/surface_inventory/mod.rs";
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

#[test]
fn authority_surface_inventory_discovers_foundational_red_fixtures() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("authority-red-inventory");
    std::fs::create_dir_all(root.join("fixtures/red")).expect("red fixture dir");
    std::fs::create_dir_all(root.join("templates")).expect("template dir");
    std::fs::write(
        root.join("fixtures/red/typed-records-over-prose-raw-path-authority-rejected-red.json"),
        "{}",
    )
    .expect("red fixture");
    std::fs::write(
        root.join("templates/RED_FIXTURES.json"),
        serde_json::to_vec(&json!([{
            "id": "typed-records-over-prose-raw-path-authority-rejected-red",
            "packet_path": "fixtures/red/typed-records-over-prose-raw-path-authority-rejected-red.json"
        }]))
        .expect("catalog json"),
    )
    .expect("catalog");
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({
            "resources": [
                "fixtures/red/typed-records-over-prose-raw-path-authority-rejected-red.json"
            ]
        }))
        .expect("manifest json"),
    )
    .expect("manifest");

    let inventory = crate::audit::law::authority_surfaces::foundational_surface_inventory(&root);
    assert_eq!(inventory["role_counts"]["red_fixture"], json!(1));
    assert!(
        inventory
            .get("rows")
            .and_then(serde_json::Value::as_array)
            .expect("rows")
            .iter()
            .any(
                |row| row.get("role").and_then(serde_json::Value::as_str) == Some("red_fixture")
                    && row.get("surface_state").and_then(serde_json::Value::as_str)
                        == Some("available")
            ),
        "{inventory}"
    );
    std::fs::remove_dir_all(root).expect("cleanup red fixture inventory");
}

#[test]
fn authority_surface_inventory_marks_unlisted_generated_artifact_blocked() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "authority-generated-inventory",
    );
    std::fs::create_dir_all(root.join("examples/generated")).expect("generated dir");
    std::fs::write(
        root.join("examples/generated/READY_FOR_MERGE.example.json"),
        "{}",
    )
    .expect("generated artifact");
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({ "resources": [] })).expect("manifest json"),
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
                == Some("generated_artifact")
                && row.get("path").and_then(serde_json::Value::as_str)
                    == Some("examples/generated/READY_FOR_MERGE.example.json")
                && row
                    .get("listed_in_package_inventory")
                    .and_then(serde_json::Value::as_bool)
                    == Some(false)
                && row.get("surface_state").and_then(serde_json::Value::as_str) == Some("blocked")),
        "{inventory}"
    );
    std::fs::remove_dir_all(root).expect("cleanup generated artifact inventory");
}
