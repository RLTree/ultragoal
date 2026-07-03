use serde_json::json;

#[test]
fn observability_control_board_shape_edges_are_explicit() {
    let mut failures = Vec::new();
    super::super::super::control::check(&json!({}), &mut failures);
    assert!(failures.contains(&"observability_control_board_missing".to_string()));

    let mut observable = super::super::inventory_fixtures::observable_inventory();
    observable["observability_control_board"]["first_incomplete"] = json!({
        "family": "commands",
        "id": "stale",
        "observability_status": "unobservable",
        "next_unobservable_surface": "stale"
    });
    failures.clear();
    super::super::super::control::check(&observable, &mut failures);
    assert!(failures.contains(&"observability_control_board_stale_first_incomplete".to_string()));

    observable["observability_control_board"]["families"]
        .as_object_mut()
        .unwrap()
        .remove("signals");
    failures.clear();
    super::super::super::control::check(&observable, &mut failures);
    assert!(
        failures
            .iter()
            .any(|item| item == "observability_control_board_family_missing:signals")
    );

    let mut mismatch = super::super::inventory_fixtures::observable_inventory();
    mismatch["command_observability_inventory"]["package digest"]["observability_status"] =
        json!("unobservable");
    mismatch["command_observability_inventory"]["package digest"]["missing_surfaces"] =
        json!(["log"]);
    mismatch["observability_control_board"]["status"] = json!("blocked");
    mismatch["observability_control_board"]["families"]["commands"]["observable"] =
        json!(super::super::super::command_inventory::REQUIRED_COMMANDS.len() - 1);
    mismatch["observability_control_board"]["families"]["commands"]["unobservable"] = json!(1);
    mismatch["observability_control_board"]["first_incomplete"] = json!({
        "family": "commands",
        "id": "package digest",
        "observability_status": "observable",
        "next_unobservable_surface": "wrong"
    });
    failures.clear();
    super::super::super::control::check(&mismatch, &mut failures);
    assert!(failures.iter().any(|item| {
        item == "observability_control_board_first_incomplete_mismatch:commands:package digest"
    }));

    let mut claim = super::super::inventory_fixtures::observable_inventory();
    claim["observability_control_board"]["claim_impact"] = json!("");
    claim["signal_inventory"]["latency"]["observability_status"] = json!("unknown");
    failures.clear();
    super::super::super::control::check(&claim, &mut failures);
    assert!(failures.contains(&"observability_control_board_claim_impact_missing".to_string()));

    let mut missing_inventory = super::super::inventory_fixtures::observable_inventory();
    missing_inventory
        .as_object_mut()
        .unwrap()
        .remove("signal_inventory");
    failures.clear();
    super::super::super::control::check(&missing_inventory, &mut failures);
    assert!(
        failures
            .iter()
            .any(|item| { item == "observability_control_board_count_mismatch:signals:total" })
    );
}

#[test]
fn observability_surface_and_operating_shape_edges_are_explicit() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("observe-shape-edges");
    let mut failures = Vec::new();
    super::super::super::surfaces::check(&root, &json!({}), &mut failures);
    assert!(
        failures
            .contains(&"observability_surface_command_observability_inventory_missing".to_string())
    );
    assert!(
        failures
            .iter()
            .any(|item| item.starts_with("observability_surface_inventory_missing:"))
    );

    let mut inventory = super::super::inventory_fixtures::observable_inventory();
    inventory["surfaces"]
        .as_array_mut()
        .unwrap()
        .push(json!("unknown"));
    inventory["surface_inventory"]["unknown"] = json!({});
    inventory["surface_inventory"]["schema parse boundaries"] = json!("bad");
    failures.clear();
    super::super::super::surfaces::check(&root, &inventory, &mut failures);
    assert!(failures.contains(&"observability_surface_inventory_unknown:unknown".to_string()));
    assert!(failures.contains(&"observability_surface_telemetry_unknown:unknown".to_string()));
    assert!(failures.iter().any(|item| {
        item == "observability_surface_telemetry_row_not_object:schema parse boundaries"
    }));

    let mut surface_rows = super::super::inventory_fixtures::observable_inventory();
    surface_rows["surface_inventory"]
        .as_object_mut()
        .unwrap()
        .remove("cli command families");
    surface_rows["surface_inventory"]["validator check families"]["observability_status"] =
        json!("invalid");
    surface_rows["surface_inventory"]["schema parse boundaries"]["observability_status"] =
        json!("partially_observable");
    surface_rows["surface_inventory"]["schema parse boundaries"]["missing_surfaces"] = json!([]);
    surface_rows["surface_inventory"]["receipt proof artifacts"] = json!({});
    failures.clear();
    super::super::super::surfaces::check(&root, &surface_rows, &mut failures);
    assert!(
        failures
            .contains(&"observability_surface_telemetry_missing:cli command families".to_string())
    );
    assert!(failures.iter().any(|item| {
        item == "observability_surface_observability_status_invalid:validator check families:invalid"
    }));
    assert!(failures.iter().any(|item| {
        item == "observability_surface_telemetry_unobservable:schema parse boundaries:partially_observable"
    }));
    assert!(failures.iter().any(|item| {
        item == "observability_surface_observability_status_missing:receipt proof artifacts"
    }));

    let mut missing_claim = super::super::inventory_fixtures::observable_inventory();
    missing_claim["surface_inventory"]["schema parse boundaries"]["observability_status"] =
        json!("partially_observable");
    missing_claim["surface_inventory"]["schema parse boundaries"]["missing_surfaces"] =
        json!(["trace"]);
    missing_claim["surface_inventory"]["schema parse boundaries"]["current_owner_surface"] =
        json!("validator");
    missing_claim["surface_inventory"]["schema parse boundaries"]["next_unobservable_surface"] =
        json!("trace");
    missing_claim["surface_inventory"]["schema parse boundaries"]["claim_impact"] = json!("");
    failures.clear();
    super::super::super::surfaces::check(&root, &missing_claim, &mut failures);
    assert!(failures.contains(
        &"observability_surface_telemetry_missing_metadata:schema parse boundaries".to_string()
    ));

    let mut surface_contract = super::super::inventory_fixtures::observable_inventory();
    surface_contract["surface_inventory"]["schema parse boundaries"]["observability_status"] =
        json!("partially_observable");
    surface_contract["surface_inventory"]["schema parse boundaries"]["observed_surfaces"] =
        json!([]);
    surface_contract["surface_inventory"]["schema parse boundaries"]["missing_surfaces"] = json!([
        "log instrumentation",
        "metric instrumentation",
        "trace instrumentation",
        "pass stdout contract",
        "fail stdout contract"
    ]);
    failures.clear();
    super::super::super::surfaces::check(&root, &surface_contract, &mut failures);
    assert!(failures.contains(
        &"observability_surface_telemetry_missing_metadata:schema parse boundaries".to_string()
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

    let mut loop_inventory = super::super::inventory_fixtures::observable_inventory();
    loop_inventory["operating_loop_inventory"]["current_digest_first"] = json!({});
    loop_inventory["signal_inventory"]["latency"] = json!({
        "observability_status": "unobservable",
        "missing_surfaces": [],
        "claim_impact": ""
    });
    failures.clear();
    super::super::super::operating::check(&root, &loop_inventory, &mut failures);
    assert!(failures.contains(
        &"observability_loop_observability_status_missing:current_digest_first".to_string()
    ));
    assert!(
        failures.contains(&"observability_signal_telemetry_missing_metadata:latency".to_string())
    );

    let mut operating_contract = super::super::inventory_fixtures::observable_inventory();
    operating_contract["operating_loop_inventory"]["current_digest_first"]["observed_surfaces"] =
        json!([
            "log instrumentation",
            "metric instrumentation",
            "trace instrumentation",
            "pass stdout contract",
            "receipt observability binding"
        ]);
    failures.clear();
    super::super::super::operating::check(&root, &operating_contract, &mut failures);
    assert!(
        failures.contains(
            &"observability_loop_telemetry_row_shape_only:current_digest_first".to_string()
        )
    );

    let mut loop_edges = super::super::inventory_fixtures::observable_inventory();
    loop_edges["operating_loop_inventory"]
        .as_object_mut()
        .unwrap()
        .remove("current_digest_first");
    loop_edges["operating_loop_inventory"]["run_failing_command_once"] = json!("bad");
    loop_edges["operating_loop_inventory"]["query_logs_metrics_traces_by_run_id"]["observability_status"] =
        json!("invalid");
    loop_edges["operating_loop_inventory"]["repair_smallest_root_cause"]["observability_status"] =
        json!("partially_observable");
    loop_edges["operating_loop_inventory"]["repair_smallest_root_cause"]["missing_surfaces"] =
        json!(["trace"]);
    loop_edges["operating_loop_inventory"]["repair_smallest_root_cause"]["claim_impact"] =
        json!("blocks_observability_product_closure");
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
        item == "observability_loop_observability_status_invalid:query_logs_metrics_traces_by_run_id:invalid"
    }));
    assert!(failures.iter().any(|item| {
        item == "observability_loop_telemetry_unobservable:repair_smallest_root_cause:partially_observable"
    }));
    assert!(failures.contains(&"observability_loop_inventory_unknown:unknown".to_string()));
    let _ = std::fs::remove_dir_all(root);
}
