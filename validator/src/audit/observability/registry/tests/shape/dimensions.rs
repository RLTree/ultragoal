use serde_json::json;

#[test]
fn dimension_inventory_shape_edges_cover_unknown_status_and_partial_rows() {
    let root = crate::self_tests::boundaries::support::temp_root("observe-dimension-branches");
    let mut inventory = super::super::support::fitted_inventory();
    inventory["validator_check_families"]
        .as_array_mut()
        .unwrap()
        .push(json!("unknown validator"));
    inventory["validator_check_inventory"]["unknown validator"] = json!({});
    inventory["validator_check_inventory"]["standards enforcement"]["fitting_status"] =
        json!("partially_fitted");
    inventory["validator_check_inventory"]["standards enforcement"]["missing_surfaces"] =
        json!(["trace instrumentation"]);
    inventory["validator_check_inventory"]["standards enforcement"]["next_unfitted_surface"] =
        json!("trace instrumentation");
    inventory["receipt_proof_inventory"]["coverage receipt"]["fitting_status"] = json!("unfitted");
    inventory["receipt_proof_inventory"]["coverage receipt"]["missing_surfaces"] = json!([]);
    inventory["receipt_proof_inventory"]["coverage receipt"]["next_unfitted_surface"] =
        json!("metric instrumentation");
    inventory["fixture_report_inventory"]["red fixture report"]["fitting_status"] =
        json!("invalid");
    inventory["package_plugin_setup_retrofit_inventory"]["schemas"]
        .as_object_mut()
        .unwrap()
        .remove("fitting_status");
    let mut failures = Vec::new();
    super::super::super::dimensions::check(&root, &inventory, &mut failures);
    assert!(failures.contains(
        &"observability_validator_check_inventory_unknown:unknown validator".to_string()
    ));
    assert!(
        failures.contains(
            &"observability_validator_check_fitting_unknown:unknown validator".to_string()
        )
    );
    assert!(failures.contains(
        &"observability_validator_check_fitting_unfitted:standards enforcement:partially_fitted"
            .to_string()
    ));
    assert!(failures.contains(
        &"observability_receipt_proof_fitting_missing_metadata:coverage receipt".to_string()
    ));
    assert!(failures.contains(
        &"observability_receipt_proof_fitting_unfitted:coverage receipt:unfitted".to_string()
    ));
    assert!(
        failures.contains(
            &"observability_fixture_report_fitting_status_invalid:red fixture report:invalid"
                .to_string()
        )
    );
    assert!(
        failures
            .contains(&"observability_package_setup_fitting_status_missing:schemas".to_string())
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn command_unfitted_metadata_checks_surfaces_and_claim_impact() {
    let root = crate::self_tests::boundaries::support::temp_root("observe-command-metadata");
    let mut inventory = super::super::support::fitted_inventory();
    inventory["fitting_inventory"]["package digest"]["fitting_status"] = json!("unfitted");
    inventory["fitting_inventory"]["package digest"]["missing_surfaces"] = json!([]);
    inventory["fitting_inventory"]["package digest"]["next_unfitted_surface"] =
        json!("trace instrumentation");
    inventory["fitting_inventory"]["source audit"]["fitting_status"] = json!("partially_fitted");
    inventory["fitting_inventory"]["source audit"]["missing_surfaces"] =
        json!(["trace instrumentation"]);
    inventory["fitting_inventory"]["source audit"]["next_unfitted_surface"] =
        json!("trace instrumentation");
    inventory["fitting_inventory"]["source audit"]
        .as_object_mut()
        .unwrap()
        .remove("claim_impact");
    let mut failures = Vec::new();
    super::super::super::fitting::check(&root, &inventory, &mut failures);
    assert!(
        failures
            .contains(&"observability_command_fitting_missing_metadata:package digest".to_string())
    );
    assert!(
        failures.contains(
            &"observability_command_fitting_unfitted:package digest:unfitted".to_string()
        )
    );
    assert!(
        failures
            .contains(&"observability_command_fitting_missing_metadata:source audit".to_string())
    );
    assert!(failures.contains(
        &"observability_command_fitting_unfitted:source audit:partially_fitted".to_string()
    ));
    let _ = std::fs::remove_dir_all(root);
}
