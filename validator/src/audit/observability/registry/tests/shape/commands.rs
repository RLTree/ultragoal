use serde_json::json;

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
    let _ = std::fs::remove_dir_all(root);
}
