use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn observability_command_inventory_shape_edges_are_explicit() {
    let root = crate::self_tests::boundaries::support::temp_root("observe-command-shape");
    let mut failures = Vec::new();
    super::super::super::command_inventory::check(&root, &json!({}), &mut failures);
    assert!(failures.contains(&"observability_command_fitting_inventory_missing".to_string()));
    assert!(
        failures
            .iter()
            .any(|item| item.starts_with("observability_command_inventory_missing:"))
    );

    let mut inventory = super::super::support::fitted_inventory();
    inventory["commands"]
        .as_array_mut()
        .unwrap()
        .push(json!("unknown"));
    inventory["fitting_inventory"]["unknown"] = json!({});
    inventory["fitting_inventory"]
        .as_object_mut()
        .unwrap()
        .remove("package digest");
    inventory["fitting_inventory"]["source audit"] = json!("bad");
    inventory["fitting_inventory"]["red fixture report"]["fitting_status"] = json!("invalid");
    inventory["fitting_inventory"]["schema validation"] = json!({});
    failures.clear();
    super::super::super::command_inventory::check(&root, &inventory, &mut failures);
    assert!(failures.contains(&"observability_command_inventory_unknown:unknown".to_string()));
    assert!(failures.contains(&"observability_command_fitting_unknown:unknown".to_string()));
    assert!(failures.contains(&"observability_command_fitting_missing:package digest".to_string()));
    assert!(
        failures.contains(&"observability_command_fitting_row_not_object:source audit".to_string())
    );
    assert!(failures.iter().any(|item| {
        item == "observability_command_fitting_status_invalid:red fixture report:invalid"
    }));
    assert!(
        failures.contains(
            &"observability_command_fitting_status_missing:schema validation".to_string()
        )
    );

    let mut missing_command = super::super::support::fitted_inventory();
    let commands = missing_command["commands"].as_array_mut().unwrap();
    commands.insert(0, json!(42));
    commands.retain(|row| row.as_str() != Some("package digest"));
    failures.clear();
    super::super::super::command_inventory::check(&root, &missing_command, &mut failures);
    assert!(
        failures.contains(&"observability_command_inventory_missing:package digest".to_string())
    );

    let mut malformed_containers = super::super::support::fitted_inventory();
    malformed_containers["commands"] = json!("not an array");
    malformed_containers["fitting_inventory"] = json!("not an object");
    malformed_containers["row_requirements"] = json!({});
    failures.clear();
    super::super::super::command_inventory::check(&root, &malformed_containers, &mut failures);
    assert!(
        failures
            .iter()
            .any(|item| item.starts_with("observability_command_inventory_missing:"))
    );
    assert!(failures.contains(&"observability_command_fitting_inventory_missing".to_string()));
    assert!(
        failures.iter().any(|item| {
            item.starts_with("observability_command_inventory_requirement_missing:")
        })
    );

    let mut partial_metadata = super::super::support::fitted_inventory();
    partial_metadata["fitting_inventory"]["package digest"]["fitting_status"] =
        json!("partially_fitted");
    partial_metadata["fitting_inventory"]["package digest"]["missing_surfaces"] = json!(["trace"]);
    partial_metadata["fitting_inventory"]["package digest"]["next_unfitted_surface"] =
        json!("trace");
    partial_metadata["fitting_inventory"]["package digest"]
        .as_object_mut()
        .unwrap()
        .remove("focused_tests");
    failures.clear();
    super::super::super::command_inventory::check(&root, &partial_metadata, &mut failures);
    assert!(
        failures
            .contains(&"observability_command_fitting_missing_metadata:package digest".to_string())
    );

    let mut row_contract = super::super::support::fitted_inventory();
    row_contract["fitting_inventory"]["package digest"]["fitted_surfaces"] = json!([
        "log instrumentation",
        "metric instrumentation",
        "trace instrumentation",
        "pass stdout contract",
        "receipt binding"
    ]);
    failures.clear();
    super::super::super::command_inventory::check(&root, &row_contract, &mut failures);
    assert!(
        failures
            .contains(&"observability_command_fitting_row_shape_only:package digest".to_string())
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn observability_dimension_inventories_are_required() {
    let root = crate::self_tests::boundaries::support::temp_root("observe-dimension-shape");
    let mut inventory = super::super::support::fitted_inventory();
    inventory["validator_check_families"]
        .as_array_mut()
        .unwrap()
        .retain(|row| row.as_str() != Some("standards enforcement"));
    inventory["validator_check_inventory"]
        .as_object_mut()
        .unwrap()
        .remove("source obligations");
    inventory["receipt_proof_inventory"]["coverage receipt"] = json!("bad");
    inventory["claim_guard_inventory"]["completion"]
        .as_object_mut()
        .unwrap()
        .remove("claim_impact");
    let mut failures = Vec::new();
    super::super::super::dimensions::check(&root, &inventory, &mut failures);
    assert!(failures.contains(
        &"observability_validator_check_inventory_missing:standards enforcement".to_string()
    ));
    assert!(
        failures.contains(
            &"observability_validator_check_fitting_missing:source obligations".to_string()
        )
    );
    assert!(failures.contains(
        &"observability_receipt_proof_fitting_row_not_object:coverage receipt".to_string()
    ));
    assert!(
        failures
            .contains(&"observability_claim_guard_fitting_row_shape_only:completion".to_string())
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn production_command_inventory_matches_required_command_authority() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let inventory = crate::json_boundary::read_json(
        &root.join("docs/generated/observability/command-inventory.json"),
    )
    .expect("production command inventory");
    let listed = inventory
        .get("commands")
        .and_then(serde_json::Value::as_array)
        .expect("commands array")
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect::<BTreeSet<_>>();
    let required = super::super::super::command_inventory::REQUIRED_COMMANDS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    assert_eq!(listed, required);
}

#[test]
fn production_command_control_board_counts_match_inventory_rows() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let inventory = crate::json_boundary::read_json(
        &root.join("docs/generated/observability/command-inventory.json"),
    )
    .expect("production command inventory");
    let command_rows = inventory
        .get("fitting_inventory")
        .and_then(serde_json::Value::as_object)
        .expect("command inventory rows");
    let mut expected = BTreeMap::from([
        ("total", command_rows.len() as u64),
        ("fitted", 0),
        ("partially_fitted", 0),
        ("unfitted", 0),
    ]);
    for row in command_rows.values() {
        if let Some(status) = row
            .get("fitting_status")
            .and_then(serde_json::Value::as_str)
        {
            *expected.entry(status).or_insert(0) += 1;
        }
    }
    let board = inventory
        .pointer("/fitting_control_board/families/commands")
        .and_then(serde_json::Value::as_object)
        .expect("command control-board counts");
    for (key, count) in expected {
        assert_eq!(
            board.get(key).and_then(serde_json::Value::as_u64),
            Some(count)
        );
    }
}

#[test]
fn control_board_matches_command_inventory_rows() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let inventory = crate::json_boundary::read_json(
        &root.join("docs/generated/observability/command-inventory.json"),
    )
    .expect("production command inventory");
    let mut failures = Vec::new();
    super::super::super::control::check(&inventory, &mut failures);
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn production_inventory_rows_account_for_required_observability_contracts() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let inventory = crate::json_boundary::read_json(
        &root.join("docs/generated/observability/command-inventory.json"),
    )
    .expect("production command inventory");
    for (family, key) in [
        ("commands", "fitting_inventory"),
        ("surfaces", "surface_inventory"),
        ("operating_loop", "operating_loop_inventory"),
        ("signals", "signal_inventory"),
        ("validator_checks", "validator_check_inventory"),
        ("receipts", "receipt_proof_inventory"),
        ("fixtures", "fixture_report_inventory"),
        ("package_setup", "package_plugin_setup_retrofit_inventory"),
        ("long_running", "long_running_path_inventory"),
        ("external_live", "external_live_path_inventory"),
        ("claim_guards", "claim_guard_inventory"),
    ] {
        let rows = inventory
            .get(key)
            .and_then(serde_json::Value::as_object)
            .expect("inventory rows");
        for (id, row) in rows {
            let object = row.as_object().expect("inventory row object");
            assert!(
                super::super::super::row_contract::complete(object),
                "{family}:{id} must account for log, metric, trace, pass stdout, fail stdout, and receipt observability binding"
            );
        }
    }
}
