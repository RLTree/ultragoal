use serde_json::json;

#[test]
fn dimension_inventory_shape_edges_cover_unknown_status_and_partial_rows() {
    let root = crate::self_tests::boundaries::support::temp_root("observe-dimension-branches");
    let mut inventory = super::super::support::observable_inventory();
    inventory["validator_check_families"]
        .as_array_mut()
        .unwrap()
        .push(json!("unknown validator"));
    inventory["validator_check_inventory"]["unknown validator"] = json!({});
    inventory["validator_check_inventory"]["standards enforcement"]["observability_status"] =
        json!("partially_observable");
    inventory["validator_check_inventory"]["standards enforcement"]["missing_surfaces"] =
        json!(["trace instrumentation"]);
    inventory["validator_check_inventory"]["standards enforcement"]["next_unobservable_surface"] =
        json!("trace instrumentation");
    inventory["receipt_proof_inventory"]["coverage receipt"]["observability_status"] =
        json!("unobservable");
    inventory["receipt_proof_inventory"]["coverage receipt"]["missing_surfaces"] = json!([]);
    inventory["receipt_proof_inventory"]["coverage receipt"]["next_unobservable_surface"] =
        json!("metric instrumentation");
    inventory["fixture_report_inventory"]["red fixture report"]["observability_status"] =
        json!("invalid");
    inventory["package_plugin_setup_retrofit_inventory"]["schemas"]
        .as_object_mut()
        .unwrap()
        .remove("observability_status");
    let mut failures = Vec::new();
    super::super::super::dimensions::check(&root, &inventory, &mut failures);
    assert!(failures.contains(
        &"observability_validator_check_inventory_unknown:unknown validator".to_string()
    ));
    assert!(failures.contains(
        &"observability_validator_check_telemetry_unknown:unknown validator".to_string()
    ));
    assert!(failures.contains(
        &"observability_validator_check_telemetry_unobservable:standards enforcement:partially_observable"
            .to_string()
    ));
    assert!(failures.contains(
        &"observability_receipt_proof_telemetry_missing_metadata:coverage receipt".to_string()
    ));
    assert!(
        failures.contains(
            &"observability_receipt_proof_telemetry_unobservable:coverage receipt:unobservable"
                .to_string()
        )
    );
    assert!(
        failures.contains(
            &"observability_fixture_report_observability_status_invalid:red fixture report:invalid"
                .to_string()
        )
    );
    assert!(
        failures.contains(
            &"observability_package_setup_observability_status_missing:schemas".to_string()
        )
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn command_unobservable_metadata_checks_surfaces_and_claim_impact() {
    let root = crate::self_tests::boundaries::support::temp_root("observe-command-metadata");
    let mut inventory = super::super::support::observable_inventory();
    inventory["command_observability_inventory"]["package digest"]["observability_status"] =
        json!("unobservable");
    inventory["command_observability_inventory"]["package digest"]["missing_surfaces"] = json!([]);
    inventory["command_observability_inventory"]["package digest"]["next_unobservable_surface"] =
        json!("trace instrumentation");
    inventory["command_observability_inventory"]["source audit"]["observability_status"] =
        json!("partially_observable");
    inventory["command_observability_inventory"]["source audit"]["missing_surfaces"] =
        json!(["trace instrumentation"]);
    inventory["command_observability_inventory"]["source audit"]["next_unobservable_surface"] =
        json!("trace instrumentation");
    inventory["command_observability_inventory"]["source audit"]
        .as_object_mut()
        .unwrap()
        .remove("claim_impact");
    let mut failures = Vec::new();
    super::super::super::command_inventory::check(&root, &inventory, &mut failures);
    assert!(
        failures.contains(
            &"observability_command_telemetry_missing_metadata:package digest".to_string()
        )
    );
    assert!(failures.contains(
        &"observability_command_telemetry_unobservable:package digest:unobservable".to_string()
    ));
    assert!(
        failures
            .contains(&"observability_command_telemetry_missing_metadata:source audit".to_string())
    );
    assert!(
        failures.contains(
            &"observability_command_telemetry_unobservable:source audit:partially_observable"
                .to_string()
        )
    );
    let _ = std::fs::remove_dir_all(root);
}
