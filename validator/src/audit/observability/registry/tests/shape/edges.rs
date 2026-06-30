use serde_json::json;

#[test]
fn observability_control_board_shape_edges_are_explicit() {
    let mut failures = Vec::new();
    super::super::super::control::check(&json!({}), &mut failures);
    assert!(failures.contains(&"observability_fitting_control_board_missing".to_string()));

    let mut fitted = super::super::support::fitted_inventory();
    fitted["fitting_control_board"]["first_incomplete"] = json!({
        "family": "commands",
        "id": "stale",
        "fitting_status": "unfitted",
        "next_unfitted_surface": "stale"
    });
    failures.clear();
    super::super::super::control::check(&fitted, &mut failures);
    assert!(
        failures
            .contains(&"observability_fitting_control_board_stale_first_incomplete".to_string())
    );

    fitted["fitting_control_board"]["families"]
        .as_object_mut()
        .unwrap()
        .remove("signals");
    failures.clear();
    super::super::super::control::check(&fitted, &mut failures);
    assert!(
        failures
            .iter()
            .any(|item| item == "observability_fitting_control_board_family_missing:signals")
    );

    let mut mismatch = super::super::support::fitted_inventory();
    mismatch["fitting_inventory"]["package digest"]["fitting_status"] = json!("unfitted");
    mismatch["fitting_inventory"]["package digest"]["missing_surfaces"] = json!(["log"]);
    mismatch["fitting_control_board"]["status"] = json!("blocked");
    mismatch["fitting_control_board"]["families"]["commands"]["fitted"] =
        json!(super::super::super::fitting::REQUIRED_COMMANDS.len() - 1);
    mismatch["fitting_control_board"]["families"]["commands"]["unfitted"] = json!(1);
    mismatch["fitting_control_board"]["first_incomplete"] = json!({
        "family": "commands",
        "id": "package digest",
        "fitting_status": "fitted",
        "next_unfitted_surface": "wrong"
    });
    failures.clear();
    super::super::super::control::check(&mismatch, &mut failures);
    assert!(failures.iter().any(|item| {
        item == "observability_fitting_control_board_first_incomplete_mismatch:commands:package digest"
    }));

    let mut claim = super::super::support::fitted_inventory();
    claim["fitting_control_board"]["claim_impact"] = json!("");
    claim["signal_inventory"]["latency"]["fitting_status"] = json!("unknown");
    failures.clear();
    super::super::super::control::check(&claim, &mut failures);
    assert!(
        failures.contains(&"observability_fitting_control_board_claim_impact_missing".to_string())
    );

    let mut missing_inventory = super::super::support::fitted_inventory();
    missing_inventory
        .as_object_mut()
        .unwrap()
        .remove("signal_inventory");
    failures.clear();
    super::super::super::control::check(&missing_inventory, &mut failures);
    assert!(failures.iter().any(|item| {
        item == "observability_fitting_control_board_count_mismatch:signals:total"
    }));
}

#[test]
fn observability_surface_and_operating_shape_edges_are_explicit() {
    let root = crate::self_tests::boundaries::support::temp_root("observe-shape-edges");
    let mut failures = Vec::new();
    super::super::super::surfaces::check(&root, &json!({}), &mut failures);
    assert!(failures.contains(&"observability_surface_fitting_inventory_missing".to_string()));
    assert!(
        failures
            .iter()
            .any(|item| item.starts_with("observability_surface_inventory_missing:"))
    );

    let mut inventory = super::super::support::fitted_inventory();
    inventory["surfaces"]
        .as_array_mut()
        .unwrap()
        .push(json!("unknown"));
    inventory["surface_inventory"]["unknown"] = json!({});
    inventory["surface_inventory"]["schema parse boundaries"] = json!("bad");
    failures.clear();
    super::super::super::surfaces::check(&root, &inventory, &mut failures);
    assert!(failures.contains(&"observability_surface_inventory_unknown:unknown".to_string()));
    assert!(failures.contains(&"observability_surface_fitting_unknown:unknown".to_string()));
    assert!(failures.iter().any(|item| {
        item == "observability_surface_fitting_row_not_object:schema parse boundaries"
    }));

    let mut surface_rows = super::super::support::fitted_inventory();
    surface_rows["surface_inventory"]
        .as_object_mut()
        .unwrap()
        .remove("cli command families");
    surface_rows["surface_inventory"]["validator check families"]["fitting_status"] =
        json!("invalid");
    surface_rows["surface_inventory"]["schema parse boundaries"]["fitting_status"] =
        json!("partially_fitted");
    surface_rows["surface_inventory"]["schema parse boundaries"]["missing_surfaces"] = json!([]);
    surface_rows["surface_inventory"]["receipt proof artifacts"] = json!({});
    failures.clear();
    super::super::super::surfaces::check(&root, &surface_rows, &mut failures);
    assert!(
        failures
            .contains(&"observability_surface_fitting_missing:cli command families".to_string())
    );
    assert!(failures.iter().any(|item| {
        item == "observability_surface_fitting_status_invalid:validator check families:invalid"
    }));
    assert!(failures.iter().any(|item| {
        item == "observability_surface_fitting_unfitted:schema parse boundaries:partially_fitted"
    }));
    assert!(failures.iter().any(|item| {
        item == "observability_surface_fitting_status_missing:receipt proof artifacts"
    }));

    let mut missing_claim = super::super::support::fitted_inventory();
    missing_claim["surface_inventory"]["schema parse boundaries"]["fitting_status"] =
        json!("partially_fitted");
    missing_claim["surface_inventory"]["schema parse boundaries"]["missing_surfaces"] =
        json!(["trace"]);
    missing_claim["surface_inventory"]["schema parse boundaries"]["current_owner_surface"] =
        json!("validator");
    missing_claim["surface_inventory"]["schema parse boundaries"]["next_unfitted_surface"] =
        json!("trace");
    missing_claim["surface_inventory"]["schema parse boundaries"]["claim_impact"] = json!("");
    failures.clear();
    super::super::super::surfaces::check(&root, &missing_claim, &mut failures);
    assert!(failures.contains(
        &"observability_surface_fitting_missing_metadata:schema parse boundaries".to_string()
    ));

    failures.clear();
    super::super::super::operating::check(&root, &json!({}), &mut failures);
    assert!(
        failures
            .iter()
            .any(|item| item.starts_with("observability_operating_doctrine_missing:"))
    );
    assert!(failures.contains(&"observability_loop_inventory_missing".to_string()));
    assert!(failures.contains(&"observability_signal_inventory_missing".to_string()));

    let mut loop_inventory = super::super::support::fitted_inventory();
    loop_inventory["operating_loop_inventory"]["current_digest_first"] = json!({});
    loop_inventory["signal_inventory"]["latency"] = json!({
        "fitting_status": "unfitted",
        "missing_surfaces": [],
        "claim_impact": ""
    });
    failures.clear();
    super::super::super::operating::check(&root, &loop_inventory, &mut failures);
    assert!(
        failures.contains(
            &"observability_loop_fitting_status_missing:current_digest_first".to_string()
        )
    );
    assert!(
        failures.contains(&"observability_signal_fitting_missing_metadata:latency".to_string())
    );

    let mut loop_edges = super::super::support::fitted_inventory();
    loop_edges["operating_loop_inventory"]
        .as_object_mut()
        .unwrap()
        .remove("current_digest_first");
    loop_edges["operating_loop_inventory"]["run_failing_command_once"] = json!("bad");
    loop_edges["operating_loop_inventory"]["query_logs_metrics_traces_by_run_id"]["fitting_status"] =
        json!("invalid");
    loop_edges["operating_loop_inventory"]["repair_smallest_root_cause"]["fitting_status"] =
        json!("partially_fitted");
    loop_edges["operating_loop_inventory"]["repair_smallest_root_cause"]["missing_surfaces"] =
        json!(["trace"]);
    loop_edges["operating_loop_inventory"]["repair_smallest_root_cause"]["claim_impact"] =
        json!("blocks_gate_92");
    loop_edges["operating_loop_inventory"]["repair_smallest_root_cause"]
        .as_object_mut()
        .unwrap()
        .remove("current_owner_surface");
    loop_edges["operating_loop_inventory"]["unknown"] = json!({});
    failures.clear();
    super::super::super::operating::check(&root, &loop_edges, &mut failures);
    assert!(
        failures
            .contains(&"observability_loop_inventory_row_missing:current_digest_first".to_string())
    );
    assert!(failures.contains(
        &"observability_loop_inventory_row_not_object:run_failing_command_once".to_string()
    ));
    assert!(failures.iter().any(|item| {
        item == "observability_loop_fitting_status_invalid:query_logs_metrics_traces_by_run_id:invalid"
    }));
    assert!(failures.iter().any(|item| {
        item == "observability_loop_fitting_unfitted:repair_smallest_root_cause:partially_fitted"
    }));
    assert!(failures.contains(&"observability_loop_inventory_unknown:unknown".to_string()));
    let _ = std::fs::remove_dir_all(root);
}
