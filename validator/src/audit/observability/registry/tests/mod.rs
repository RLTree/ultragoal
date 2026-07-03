use super::*;
use serde_json::json;
use std::fs;

mod dimension_inventory;
mod inventory_fixtures;
mod proof;
mod receipts;
mod shape;
use inventory_fixtures::*;

#[test]
fn observability_registry_accepts_fully_observable_inventory() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("observe-registry");
    write_registry_root(&root, observable_inventory());
    let mut failures = Vec::new();
    check(&root, &mut failures);
    assert_eq!(
        failures,
        vec![format!(
            "observability_missing_valid_fixture:fixtures/mandatory-law-surfaces/valid/{}.json",
            super::super::LAW
        )]
    );
    write_valid_fixture(&root);
    failures.clear();
    check(&root, &mut failures);
    assert!(failures.is_empty(), "{failures:?}");
    fs::remove_dir_all(root).expect("cleanup registry");
}

#[test]
fn observability_registry_rejects_unobservable_and_row_shape_inventory() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("observe-roundtrip-registry");
    let mut inventory = observable_inventory();
    inventory["command_observability_inventory"]["package digest"] = json!({
        "observability_status": "unobservable",
        "observed_surfaces": [],
        "missing_surfaces": ["log"],
        "validator_check_id": super::super::LAW,
        "focused_tests": [],
        "receipt_paths": [],
        "live_query_proof_paths": [],
        "claim_impact": "blocks_observability_product_closure"
    });
    write_registry_root(&root, inventory);
    write_valid_fixture(&root);
    let mut failures = Vec::new();
    check(&root, &mut failures);
    assert!(failures.iter().any(|item| {
        item == "observability_command_telemetry_unobservable:package digest:unobservable"
    }));

    let mut row_shape = observable_inventory();
    row_shape["command_observability_inventory"]["source audit"] = json!({
        "observability_status": "observable",
        "observed_surfaces": ["log"],
        "missing_surfaces": [],
        "validator_check_id": super::super::LAW,
        "focused_tests": [],
        "receipt_paths": ["validation_artifacts/ultragoal-audit/validator-receipt.json"],
        "live_query_proof_paths": [],
        "claim_impact": "claims_complete"
    });
    write_inventory(&root, row_shape);
    failures.clear();
    check(&root, &mut failures);
    assert!(
        failures
            .iter()
            .any(|item| { item == "observability_command_telemetry_row_shape_only:source audit" })
    );

    let mut surface_shape = observable_inventory();
    surface_shape["surface_inventory"]["validator check families"] = json!({
        "observability_status": "observable",
        "observed_surfaces": ["log"],
        "missing_surfaces": [],
        "validator_check_id": super::super::LAW,
        "focused_tests": [],
        "receipt_paths": ["validation_artifacts/observability/command-roundtrip/surface-validator-check-families.json"],
        "live_query_proof_paths": [],
        "claim_impact": "claims_complete"
    });
    write_inventory(&root, surface_shape);
    failures.clear();
    check(&root, &mut failures);
    assert!(failures.iter().any(|item| {
        item == "observability_surface_telemetry_row_shape_only:validator check families"
    }));

    let mut loop_shape = observable_inventory();
    loop_shape["operating_loop_inventory"]["query_logs_metrics_traces_by_run_id"] = json!({
        "observability_status": "observable",
        "observed_surfaces": ["log"],
        "missing_surfaces": [],
        "validator_check_id": super::super::LAW,
        "focused_tests": [],
        "receipt_paths": ["validation_artifacts/observability/command-roundtrip/loop-query_logs_metrics_traces_by_run_id.json"],
        "live_query_proof_paths": [],
        "claim_impact": "claims_complete"
    });
    write_inventory(&root, loop_shape);
    failures.clear();
    check(&root, &mut failures);
    assert!(failures.iter().any(|item| {
        item == "observability_loop_telemetry_row_shape_only:query_logs_metrics_traces_by_run_id"
    }));

    let mut signal_unobservable = observable_inventory();
    signal_unobservable["signal_inventory"]["saturation"] = json!({
        "observability_status": "partially_observable",
        "observed_surfaces": ["metric names"],
        "missing_surfaces": ["queue and cache saturation query proof"],
        "validator_check_id": super::super::LAW,
        "focused_tests": [],
        "receipt_paths": [],
        "live_query_proof_paths": [],
        "claim_impact": "blocks_observability_product_closure"
    });
    write_inventory(&root, signal_unobservable);
    failures.clear();
    check(&root, &mut failures);
    assert!(failures.iter().any(|item| {
        item == "observability_signal_telemetry_unobservable:saturation:partially_observable"
    }));
    fs::remove_dir_all(root).expect("cleanup fit registry");
}

#[test]
fn observability_registry_rejects_pass_shaped_control_board() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("observe-control-board");
    let mut inventory = observable_inventory();
    inventory["command_observability_inventory"]["source audit"] = json!({
        "observability_status": "partially_observable",
        "observed_surfaces": ["log"],
        "missing_surfaces": ["metrics", "traces"],
        "validator_check_id": super::super::LAW,
        "focused_tests": ["source_audit_observability_receipt_blocks_claims_on_failed_audit"],
        "receipt_paths": [],
        "live_query_proof_paths": [],
        "current_owner_surface": "command:source audit",
        "next_unobservable_surface": "metrics",
        "claim_impact": "blocks_observability_product_closure"
    });
    inventory["observability_control_board"]["status"] = json!("observable");
    inventory["observability_control_board"]["families"]["commands"]["observable"] =
        json!(super::command_inventory::REQUIRED_COMMANDS.len());
    inventory["observability_control_board"]["families"]["commands"]["partially_observable"] =
        json!(0);
    write_registry_root(&root, inventory);
    write_valid_fixture(&root);
    let mut failures = Vec::new();
    check(&root, &mut failures);
    assert!(
        failures
            .iter()
            .any(|item| { item == "observability_control_board_status_mismatch:blocked" })
    );
    assert!(
        failures.iter().any(|item| {
            item == "observability_control_board_count_mismatch:commands:observable"
        })
    );
    assert!(failures.iter().any(|item| {
        item == "observability_control_board_first_incomplete_missing:commands:source audit"
    }));
    fs::remove_dir_all(root).expect("cleanup control board");
}

#[test]
fn observability_control_board_uses_required_command_order() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("observe-control-board-order");
    let mut inventory = observable_inventory();
    inventory["command_observability_inventory"]["source audit"] = json!({
        "observability_status": "partially_observable",
        "observed_surfaces": ["log", "receipt", "query"],
        "missing_surfaces": ["pass/fail stdout contract"],
        "validator_check_id": super::super::LAW,
        "focused_tests": ["source_audit_observability_receipt_blocks_claims_on_failed_audit"],
        "receipt_paths": [],
        "live_query_proof_paths": [],
        "current_owner_surface": "command:source audit",
        "next_unobservable_surface": "pass/fail stdout contract",
        "claim_impact": "blocks_observability_product_closure"
    });
    inventory["command_observability_inventory"]["archive build"] = json!({
        "observability_status": "unobservable",
        "observed_surfaces": [],
        "missing_surfaces": ["log"],
        "validator_check_id": super::super::LAW,
        "focused_tests": [],
        "receipt_paths": [],
        "live_query_proof_paths": [],
        "current_owner_surface": "command:archive build",
        "next_unobservable_surface": "log",
        "claim_impact": "blocks_observability_product_closure"
    });
    inventory["observability_control_board"]["status"] = json!("blocked");
    inventory["observability_control_board"]["families"]["commands"]["observable"] =
        json!(super::command_inventory::REQUIRED_COMMANDS.len() - 2);
    inventory["observability_control_board"]["families"]["commands"]["partially_observable"] =
        json!(1);
    inventory["observability_control_board"]["families"]["commands"]["unobservable"] = json!(1);
    inventory["observability_control_board"]["first_incomplete"] = json!({
        "family": "commands",
        "id": "source audit",
        "observability_status": "partially_observable",
        "next_unobservable_surface": "pass/fail stdout contract"
    });
    write_registry_root(&root, inventory);
    write_valid_fixture(&root);
    let mut failures = Vec::new();
    check(&root, &mut failures);
    assert!(
        !failures
            .iter()
            .any(|item| item.starts_with("observability_control_board_")),
        "{failures:?}"
    );
    fs::remove_dir_all(root).expect("cleanup control board order");
}
