use super::*;
use serde_json::{Map, Value, json};
use std::fs;

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
    let root = crate::self_tests::boundaries::support::temp_root("observe-fit-registry");
    let mut inventory = fitted_inventory();
    inventory["fitting_inventory"]["package digest"] = json!({
        "fitting_status": "unfitted",
        "fitted_surfaces": [],
        "missing_surfaces": ["log"],
        "validator_check_id": super::super::LAW,
        "focused_tests": [],
        "receipt_paths": [],
        "live_query_proof_paths": [],
        "claim_impact": "blocks_gate_92"
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
    fs::remove_dir_all(root).expect("cleanup fit registry");
}

fn write_registry_root(root: &std::path::Path, inventory: Value) {
    fs::create_dir_all(root.join("templates/agent-standards")).expect("standards");
    fs::create_dir_all(root.join("docs/generated/observability")).expect("inventory");
    fs::create_dir_all(root.join("docs")).expect("docs");
    fs::create_dir_all(root.join("fixtures/mandatory-law-surfaces/valid")).expect("fixtures");
    crate::json_boundary::write_json(
        &root.join("templates/agent-standards/enforcement.json"),
        &json!({"rows":[{"id": super::super::LAW}]}),
    )
    .expect("standards json");
    crate::json_boundary::write_json(
        &root.join("docs/source-obligation-matrix.json"),
        &json!({"obligations":[{"obligation_id": super::super::LAW}]}),
    )
    .expect("obligation json");
    crate::json_boundary::write_json(
        &root.join("docs/foundational-law-traceability.json"),
        &json!({"entries":[{"id": super::super::LAW}]}),
    )
    .expect("trace json");
    write_inventory(root, inventory);
}

fn write_inventory(root: &std::path::Path, inventory: Value) {
    crate::json_boundary::write_json(
        &root.join("docs/generated/observability/command-inventory.json"),
        &inventory,
    )
    .expect("inventory json");
}

fn write_valid_fixture(root: &std::path::Path) {
    fs::write(
        root.join(format!(
            "fixtures/mandatory-law-surfaces/valid/{}.json",
            super::super::LAW
        )),
        "{}",
    )
    .expect("valid fixture");
}

fn fitted_inventory() -> Value {
    let mut rows = Map::new();
    for command in fitting::REQUIRED_COMMANDS {
        rows.insert((*command).to_string(), fitted_row());
    }
    json!({
        "commands": fitting::REQUIRED_COMMANDS,
        "row_requirements": {
            "log_instrumentation": true,
            "metric_instrumentation": true,
            "trace_instrumentation": true,
            "pass_output_contract": true,
            "fail_output_contract": true,
            "receipt_observability_binding": true,
            "focused_tests": true,
            "claim_impact_mapping": true,
            "same_candidate_query_proof": true,
            "validator_enforced": true
        },
        "fitting_inventory": rows
    })
}

fn fitted_row() -> Value {
    json!({
        "fitting_status": "fitted",
        "fitted_surfaces": ["log", "metric", "trace", "stdout", "receipt"],
        "missing_surfaces": [],
        "validator_check_id": super::super::LAW,
        "focused_tests": ["observability_registry_accepts_fully_fitted_inventory"],
        "receipt_paths": ["validation_artifacts/observability/observe-prove.json"],
        "live_query_proof_paths": ["validation_artifacts/observability/observe-logs-query.json"],
        "claim_impact": "supports_gate_92_when_same_candidate"
    })
}
