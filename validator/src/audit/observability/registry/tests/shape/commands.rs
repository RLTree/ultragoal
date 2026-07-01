use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn observability_command_inventory_shape_edges_are_explicit() {
    let root = crate::self_tests::boundaries::support::temp_root("observe-command-shape");
    let mut failures = Vec::new();
    super::super::super::fitting::check(&root, &json!({}), &mut failures);
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
    super::super::super::fitting::check(&root, &inventory, &mut failures);
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
    super::super::super::fitting::check(&root, &partial_metadata, &mut failures);
    assert!(
        failures
            .contains(&"observability_command_fitting_missing_metadata:package digest".to_string())
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
    let required = super::super::super::fitting::REQUIRED_COMMANDS
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
    let fitting = inventory
        .get("fitting_inventory")
        .and_then(serde_json::Value::as_object)
        .expect("fitting inventory");
    let mut expected = BTreeMap::from([
        ("total", fitting.len() as u64),
        ("fitted", 0),
        ("partially_fitted", 0),
        ("unfitted", 0),
    ]);
    for row in fitting.values() {
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
