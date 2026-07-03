use super::super::inventory_fixtures::{
    observable_inventory, write_registry_root, write_valid_fixture,
};

#[test]
fn inventory_requires_owner_and_next_surface_tracking_together() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "observe-proof-owner-without-next",
    );
    write_registry_root(&root, observable_inventory());
    write_valid_fixture(&root);
    let path = root.join("docs/generated/observability/command-inventory.json");
    let mut inventory = crate::json_boundary::read_json(&path).unwrap();
    inventory["command_observability_inventory"]["package digest"]
        .as_object_mut()
        .unwrap()
        .remove("next_unobservable_surface");
    crate::json_boundary::write_json(&path, &inventory).unwrap();

    let failures = super::super::command_inventory_failures(&root);

    assert!(
        failures.iter().any(|item| item
            .starts_with("observability_command_telemetry_row_shape_only:package digest")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}
