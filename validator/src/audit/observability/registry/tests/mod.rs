use super::*;
use serde_json::json;
use std::fs;

mod dimension_support;
mod proof;
mod receipts;
mod shape;
mod support;
use support::*;

#[test]
fn observability_registry_accepts_fully_fitted_inventory() {
    let root = crate::self_tests::boundaries::support::temp_root("observe-registry");
    write_registry_root(&root, fitted_inventory());
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
fn observability_registry_rejects_unfitted_and_row_shape_inventory() {
    let root = crate::self_tests::boundaries::support::temp_root("observe-roundtrip-registry");
    let mut inventory = fitted_inventory();
    inventory["fitting_inventory"]["package digest"] = json!({
        "fitting_status": "unfitted",
        "fitted_surfaces": [],
        "missing_surfaces": ["log"],
        "validator_check_id": super::super::LAW,
        "focused_tests": [],
        "receipt_paths": [],
        "live_query_proof_paths": [],
        "claim_impact": "blocks_observability_gate"
    });
    write_registry_root(&root, inventory);
    write_valid_fixture(&root);
    let mut failures = Vec::new();
    check(&root, &mut failures);
    assert!(
        failures.iter().any(|item| {
            item == "observability_command_fitting_unfitted:package digest:unfitted"
        })
    );

    let mut row_shape = fitted_inventory();
    row_shape["fitting_inventory"]["source audit"] = json!({
        "fitting_status": "fitted",
        "fitted_surfaces": ["log"],
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
            .any(|item| { item == "observability_command_fitting_row_shape_only:source audit" })
    );

    let mut surface_shape = fitted_inventory();
    surface_shape["surface_inventory"]["validator check families"] = json!({
        "fitting_status": "fitted",
        "fitted_surfaces": ["log"],
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
        item == "observability_surface_fitting_row_shape_only:validator check families"
    }));

    let mut loop_shape = fitted_inventory();
    loop_shape["operating_loop_inventory"]["query_logs_metrics_traces_by_run_id"] = json!({
        "fitting_status": "fitted",
        "fitted_surfaces": ["log"],
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
        item == "observability_loop_fitting_row_shape_only:query_logs_metrics_traces_by_run_id"
    }));

    let mut signal_unfitted = fitted_inventory();
    signal_unfitted["signal_inventory"]["saturation"] = json!({
        "fitting_status": "partially_fitted",
        "fitted_surfaces": ["metric names"],
        "missing_surfaces": ["queue and cache saturation query proof"],
        "validator_check_id": super::super::LAW,
        "focused_tests": [],
        "receipt_paths": [],
        "live_query_proof_paths": [],
        "claim_impact": "blocks_observability_gate"
    });
    write_inventory(&root, signal_unfitted);
    failures.clear();
    check(&root, &mut failures);
    assert!(failures.iter().any(|item| {
        item == "observability_signal_fitting_unfitted:saturation:partially_fitted"
    }));
    fs::remove_dir_all(root).expect("cleanup fit registry");
}

#[test]
fn observability_registry_rejects_pass_shaped_control_board() {
    let root = crate::self_tests::boundaries::support::temp_root("observe-control-board");
    let mut inventory = fitted_inventory();
    inventory["fitting_inventory"]["source audit"] = json!({
        "fitting_status": "partially_fitted",
        "fitted_surfaces": ["log"],
        "missing_surfaces": ["metrics", "traces"],
        "validator_check_id": super::super::LAW,
        "focused_tests": ["source_audit_observability_receipt_blocks_claims_on_failed_audit"],
        "receipt_paths": [],
        "live_query_proof_paths": [],
        "current_owner_surface": "command:source audit",
        "next_unfitted_surface": "metrics",
        "claim_impact": "blocks_observability_gate"
    });
    inventory["fitting_control_board"]["status"] = json!("fitted");
    inventory["fitting_control_board"]["families"]["commands"]["fitted"] =
        json!(super::command_inventory::REQUIRED_COMMANDS.len());
    inventory["fitting_control_board"]["families"]["commands"]["partially_fitted"] = json!(0);
    write_registry_root(&root, inventory);
    write_valid_fixture(&root);
    let mut failures = Vec::new();
    check(&root, &mut failures);
    assert!(
        failures
            .iter()
            .any(|item| { item == "observability_fitting_control_board_status_mismatch:blocked" })
    );
    assert!(failures.iter().any(|item| {
        item == "observability_fitting_control_board_count_mismatch:commands:fitted"
    }));
    assert!(failures.iter().any(|item| {
        item == "observability_fitting_control_board_first_incomplete_missing:commands:source audit"
    }));
    fs::remove_dir_all(root).expect("cleanup control board");
}

#[test]
fn observability_control_board_uses_required_command_order() {
    let root = crate::self_tests::boundaries::support::temp_root("observe-control-board-order");
    let mut inventory = fitted_inventory();
    inventory["fitting_inventory"]["source audit"] = json!({
        "fitting_status": "partially_fitted",
        "fitted_surfaces": ["log", "receipt", "query"],
        "missing_surfaces": ["pass/fail stdout contract"],
        "validator_check_id": super::super::LAW,
        "focused_tests": ["source_audit_observability_receipt_blocks_claims_on_failed_audit"],
        "receipt_paths": [],
        "live_query_proof_paths": [],
        "current_owner_surface": "command:source audit",
        "next_unfitted_surface": "pass/fail stdout contract",
        "claim_impact": "blocks_observability_gate"
    });
    inventory["fitting_inventory"]["archive build"] = json!({
        "fitting_status": "unfitted",
        "fitted_surfaces": [],
        "missing_surfaces": ["log"],
        "validator_check_id": super::super::LAW,
        "focused_tests": [],
        "receipt_paths": [],
        "live_query_proof_paths": [],
        "current_owner_surface": "command:archive build",
        "next_unfitted_surface": "log",
        "claim_impact": "blocks_observability_gate"
    });
    inventory["fitting_control_board"]["status"] = json!("blocked");
    inventory["fitting_control_board"]["families"]["commands"]["fitted"] =
        json!(super::command_inventory::REQUIRED_COMMANDS.len() - 2);
    inventory["fitting_control_board"]["families"]["commands"]["partially_fitted"] = json!(1);
    inventory["fitting_control_board"]["families"]["commands"]["unfitted"] = json!(1);
    inventory["fitting_control_board"]["first_incomplete"] = json!({
        "family": "commands",
        "id": "source audit",
        "fitting_status": "partially_fitted",
        "next_unfitted_surface": "pass/fail stdout contract"
    });
    write_registry_root(&root, inventory);
    write_valid_fixture(&root);
    let mut failures = Vec::new();
    check(&root, &mut failures);
    assert!(
        !failures
            .iter()
            .any(|item| item.starts_with("observability_fitting_control_board_")),
        "{failures:?}"
    );
    fs::remove_dir_all(root).expect("cleanup control board order");
}
