use super::surface_fixtures::{
    expect_failure, production_law, write_json, write_specific_red_fixture,
};
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn mandatory_law_production_binding_rejects_row_shape_substitutes() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "mandatory-law-production-edges",
    );
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    write_specific_red_fixture(&root, "schema-valid-red", "schema-valid", "schema_dispatch");
    assert!(
        crate::audit::mandatory::law::surfaces::receipt_value_failures(
            &root,
            &production_law("schema-valid")
        )
        .is_empty()
    );

    write_json(
        &root.join("fixtures/red/schema-valid-red.json"),
        &json!({
            "schema": "harness-ultragoal.red-packet.v1",
            "id": "schema-valid-red",
            "expected_failure": {
                "check_id": "schema-valid",
                "error": "mandatory_law_specific_guard_not_enforced:schema-valid:schema_dispatch"
            },
            "base_fixture_path":
                "fixtures/mandatory-law-surfaces/valid/schema-valid.json",
            "json_patch": [{
                "op": "replace",
                "path": "/law_specific/unrelated_guard",
                "value": false
            }],
            "materialization": {
                "expected_validation_layer": "package",
                "first_failure_must_match_expected": true,
                "post_patch_schema_valid": true
            },
            "preconditions": [{
                "exists": true,
                "path": "/law_specific/unrelated_guard"
            }],
            "postconditions": [{
                "expectation":
                    "mandatory_law_specific_guard_not_enforced:schema-valid:schema_dispatch",
                "path": "/law_specific/unrelated_guard"
            }]
        }),
    );
    expect_failure(
        &root,
        &production_law("schema-valid"),
        "mandatory_law_specific_guard_missing_red_fixture:schema-valid:schema_dispatch",
    );
    write_specific_red_fixture(&root, "schema-valid-red", "schema-valid", "schema_dispatch");

    let mut wrong_valid = production_law("schema-valid");
    wrong_valid["valid_fixture_path"] = json!("fixtures/valid/schema-valid.json");
    expect_failure(
        &root,
        &wrong_valid,
        "mandatory_law_green_fixture_not_law_bound:schema-valid",
    );

    let mut missing_red = production_law("schema-valid");
    missing_red["red_fixture_ids"] = json!(["schema-valid"]);
    expect_failure(
        &root,
        &missing_red,
        "mandatory_law_missing_real_red_fixture:schema-valid",
    );

    let mut unbound_guard = production_law("schema-valid");
    unbound_guard["red_fixture_ids"] = json!(["schema-valid-row-shape-red"]);
    write_json(
        &root.join("fixtures/red/schema-valid-row-shape-red.json"),
        &json!({
            "schema": "harness-ultragoal.red-packet.v1",
            "id": "schema-valid-row-shape-red",
            "expected_failure": {
                "check_id": "schema-valid",
                "error": "schema_valid_row_shape_only"
            }
        }),
    );
    expect_failure(
        &root,
        &unbound_guard,
        "mandatory_law_specific_guard_missing_red_fixture:schema-valid:schema_dispatch",
    );

    let mut id_mismatch = production_law("schema-valid");
    id_mismatch["red_fixture_ids"] = json!(["schema-valid-id-mismatch-red"]);
    write_json(
        &root.join("fixtures/red/schema-valid-id-mismatch-red.json"),
        &json!({
            "schema": "harness-ultragoal.red-packet.v1",
            "id": "different-id",
            "expected_failure": {
                "check_id": "schema-valid",
                "error": "mandatory_law_specific_guard_not_enforced:schema-valid:schema_dispatch"
            }
        }),
    );
    expect_failure(
        &root,
        &id_mismatch,
        "mandatory_law_specific_guard_missing_red_fixture:schema-valid:schema_dispatch",
    );

    let mut missing_expected = production_law("schema-valid");
    missing_expected["red_fixture_ids"] = json!(["schema-valid-missing-expected-red"]);
    write_json(
        &root.join("fixtures/red/schema-valid-missing-expected-red.json"),
        &json!({
            "schema": "harness-ultragoal.red-packet.v1",
            "id": "schema-valid-missing-expected-red"
        }),
    );
    expect_failure(
        &root,
        &missing_expected,
        "mandatory_law_specific_guard_missing_red_fixture:schema-valid:schema_dispatch",
    );

    let mut production_error_bound = production_law("schema-valid");
    production_error_bound["law_specific"] = json!({"schema_valid_production_error": true});
    production_error_bound["red_fixture_ids"] = json!(["schema-valid-production-red"]);
    write_json(
        &root.join("fixtures/red/schema-valid-production-red.json"),
        &json!({
            "schema": "harness-ultragoal.red-packet.v1",
            "id": "schema-valid-production-red",
            "expected_failure": {
                "check_id": "schema-valid",
                "error": "schema_valid_production_error"
            }
        }),
    );
    expect_failure(
        &root,
        &production_error_bound,
        "mandatory_law_specific_guard_missing_red_fixture:schema-valid:schema_valid_production_error",
    );

    let law = production_law("schema-valid");
    let mut current_failures = BTreeMap::new();
    current_failures.insert("schema-valid".to_string(), Vec::new());
    assert!(
        crate::audit::mandatory::law::surfaces::current_check_failures_for_test(
            &law,
            "schema-valid",
            &current_failures,
        )
        .is_empty()
    );
    current_failures.insert(
        "schema-valid".to_string(),
        vec!["schema failure".to_string()],
    );
    let failures = crate::audit::mandatory::law::surfaces::current_check_failures_for_test(
        &law,
        "schema-valid",
        &current_failures,
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure
                == "mandatory_law_current_check_not_pass:schema-valid:schema-valid"),
        "{failures:?}"
    );
    let missing_failures = BTreeMap::new();
    let failures = crate::audit::mandatory::law::surfaces::current_check_failures_for_test(
        &law,
        "schema-valid",
        &missing_failures,
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure
                == "mandatory_law_current_check_missing:schema-valid:schema-valid"),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup mandatory law production edges");
}
