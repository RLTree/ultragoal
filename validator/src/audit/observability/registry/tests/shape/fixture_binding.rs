use serde_json::json;

#[test]
fn observable_rows_reject_unbound_fixture_labels() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("observe-fixture-binding");
    let mut inventory = super::super::inventory_fixtures::observable_inventory();
    inventory["command_observability_inventory"]["package digest"]["red_fixtures"] =
        json!(["unbound_red_fixture_label"]);

    let mut failures = Vec::new();
    super::super::super::command_inventory::check(&root, &inventory, &mut failures);
    assert!(
        failures
            .contains(&"observability_command_telemetry_row_shape_only:package digest".to_string())
    );
    let _ = std::fs::remove_dir_all(root);
}
