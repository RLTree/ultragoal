#[test]
fn final_packet_inventory_records_production_proof_and_fixture_binding() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let inventory = crate::json_boundary::read_json(
        &root.join("docs/generated/observability/command-inventory.json"),
    )
    .expect("command inventory");
    let row = inventory
        .pointer("/fitting_inventory/final-packet prove")
        .expect("final-packet prove row");

    assert_eq!(row["fitting_status"], "fitted");
    assert_eq!(row["next_unfitted_surface"], "none");
    assert_eq!(row["missing_surfaces"], serde_json::json!([]));

    assert_array_contains(
        row,
        "receipt_paths",
        "validation_artifacts/review/final-packet-proof.json",
    );
    assert_array_contains(
        row,
        "same_candidate_query_proof_paths",
        "validation_artifacts/observability/final-packet-prove-explain-failure.json",
    );
    assert_array_contains(
        row,
        "focused_tests",
        "final_packet_receipt_builds_fail_closed_and_green_paths",
    );
    assert_array_contains(
        row,
        "red_fixtures",
        "final_packet_receipt_builds_fail_closed_and_green_paths",
    );
    assert_array_contains(
        row,
        "green_fixtures",
        "final_packet_receipt_builds_fail_closed_and_green_paths",
    );
    assert_array_contains(
        row,
        "tamper_fixtures",
        "final_packet_proof_requires_same_candidate_dereferenced_packet",
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
