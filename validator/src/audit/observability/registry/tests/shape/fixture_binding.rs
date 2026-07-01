use serde_json::json;

#[test]
fn fitted_rows_reject_unbound_fixture_labels() {
    let root = crate::self_tests::boundaries::support::temp_root("observe-fixture-binding");
    let mut inventory = super::super::support::fitted_inventory();
    inventory["fitting_inventory"]["package digest"]["red_fixtures"] =
        json!(["unbound_red_fixture_label"]);

    let mut failures = Vec::new();
    super::super::super::fitting::check(&root, &inventory, &mut failures);
    assert!(
        failures
            .contains(&"observability_command_fitting_row_shape_only:package digest".to_string())
    );
    let _ = std::fs::remove_dir_all(root);
}
