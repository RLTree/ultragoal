#[test]
fn registry_probe_inventory_records_command_roundtrip_and_fixture_binding() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let inventory = crate::json_boundary::read_json(
        &root.join("docs/generated/observability/command-inventory.json"),
    )
    .expect("command inventory");

    for pointer in [
        "/command_observability_inventory/registry probe",
        "/long_running_path_inventory/registry probe",
    ] {
        let row = inventory.pointer(pointer).expect("registry probe row");
        assert_eq!(row["observability_status"], "observable");
        if pointer.contains("long_running_path_inventory") {
            assert_eq!(row["operation"], "registry_probe");
        }
        assert_eq!(row["next_unobservable_surface"], "none");
        assert_eq!(row["missing_surfaces"], serde_json::json!([]));
        assert_array_contains(
            row,
            "receipt_paths",
            "validation_artifacts/cli/registry-probe-receipt.json",
        );
        assert_array_contains(
            row,
            "same_candidate_query_proof_paths",
            "validation_artifacts/observability/registry-probe-explain-failure.json",
        );
        assert_array_contains(
            row,
            "focused_tests",
            "registry_probe_reports_registry_surface_without_packet_circularity",
        );
        assert_array_contains(
            row,
            "red_fixtures",
            "registry_probe_reports_registry_surface_without_packet_circularity",
        );
        assert_array_contains(
            row,
            "green_fixtures",
            "registry_probe_preserves_existing_live_same_surface_pass",
        );
        assert_array_contains(
            row,
            "tamper_fixtures",
            "active_registry_exposure_requires_live_same_surface_observation_provenance",
        );
        assert!(
            row["claim_impact"]
                .as_str()
                .is_some_and(|text| text.contains("blocks_registry_reviewer")),
            "{row}"
        );
    }

    let board = inventory
        .pointer("/observability_control_board/first_incomplete")
        .expect("first incomplete");
    assert_eq!(board["id"], "install audit");
    assert_eq!(board["observability_status"], "partially_observable");
    assert_eq!(
        board["next_unobservable_surface"],
        "red/green/tamper fixture proof"
    );
}

fn assert_array_contains(row: &serde_json::Value, key: &str, expected: &str) {
    assert!(
        row[key]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some(expected)),
        "{key}: {row}"
    );
}
