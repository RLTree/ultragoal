#[test]
fn namespace_command_inventory_binds_red_green_tamper_proof() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let inventory = crate::json_boundary::read_json(
        &root.join("docs/generated/observability/command-inventory.json"),
    )
    .expect("command inventory");
    let row = inventory
        .pointer("/command_observability_inventory/namespace check")
        .expect("namespace check row");
    assert_eq!(row["observability_status"], "observable");
    assert_eq!(row["missing_surfaces"].as_array().unwrap().len(), 0);
    assert_eq!(row["next_unobservable_surface"], "none");
    for test_name in [
        "namespace_command_fails_bad_namespace_with_repair_fields",
        "namespace_command_writes_pass_observability_receipt",
        "validator_source_namespace_red_packets_fail_for_intended_errors",
    ] {
        assert!(
            row["focused_tests"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item.as_str() == Some(test_name)),
            "{test_name}"
        );
    }
    assert_eq!(
        row["red_fixtures"].as_array().unwrap()[0],
        "namespace_command_fails_bad_namespace_with_repair_fields"
    );
    assert_eq!(
        row["green_fixtures"].as_array().unwrap()[0],
        "namespace_command_writes_pass_observability_receipt"
    );
    assert_eq!(
        row["tamper_fixtures"].as_array().unwrap()[0],
        "validator_source_namespace_red_packets_fail_for_intended_errors"
    );
}
