const PRODUCTION_PROOF: &str =
    "validation_artifacts/observability/review-target-build-production-proof.json";
const EXPLAIN_FAILURE: &str =
    "validation_artifacts/observability/review-target-build-explain-failure.json";

#[test]
fn review_target_inventory_records_production_proof_and_fixture_binding() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let inventory = crate::json_boundary::read_json(
        &root.join("docs/generated/observability/command-inventory.json"),
    )
    .expect("command inventory");
    let row = inventory
        .pointer("/fitting_inventory/review-target build")
        .expect("review-target row");
    assert_eq!(row["fitting_status"], "fitted");
    assert_eq!(row["next_unfitted_surface"], "none");
    assert_eq!(row["missing_surfaces"], serde_json::json!([]));
    assert_array_contains(row, "receipt_paths", PRODUCTION_PROOF);
    assert_array_contains(row, "same_candidate_query_proof_paths", EXPLAIN_FAILURE);
    assert_array_contains(
        row,
        "focused_tests",
        "observe_snapshot_accepts_pass_target_explain_without_failure_where_why",
    );
    assert_array_contains(
        row,
        "red_fixtures",
        "review_target_command_emits_fail_closed_observability",
    );
    assert_array_contains(
        row,
        "green_fixtures",
        "review_target_command_emits_pass_observability",
    );
    assert_array_contains(
        row,
        "tamper_fixtures",
        "review_target_observability_receipt_path_validation_is_fail_closed",
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
