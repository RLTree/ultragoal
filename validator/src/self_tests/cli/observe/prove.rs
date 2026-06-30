use crate::cli::observe;
use serde_json::json;
use std::fs;

#[test]
fn observe_prove_rejects_incomplete_fitting_inventory_after_stack_passes() {
    let root = super::minimal_root("observe-prove-fitting-incomplete");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let health = command(&["observe", "stack", "health"]);
    let health_receipt = observe::stack::health_receipt(
        &root,
        &health,
        vec![json!({"service":"victorialogs","status":"pass"})],
    )
    .expect("health");
    crate::json_boundary::write_json(&root.join(health.operation.receipt_rel()), &health_receipt)
        .expect("write health");
    let smoke = command(&["observe", "stack", "smoke"]);
    let smoke_receipt =
        observe::stack::smoke_receipt(&root, &smoke, &candidate, true, true, true).expect("smoke");
    crate::json_boundary::write_json(&root.join(smoke.operation.receipt_rel()), &smoke_receipt)
        .expect("write smoke");
    fs::create_dir_all(root.join("docs/generated/observability")).expect("inventory parent");
    crate::json_boundary::write_json(
        &root.join("docs/generated/observability/command-inventory.json"),
        &json!({
            "commands": ["observe prove"],
            "fitting_inventory": {
                "observe prove": {
                    "fitting_status": "partially_fitted",
                    "missing_surfaces": ["all commands fitted"],
                    "claim_impact": "blocks_gate_92"
                }
            }
        }),
    )
    .expect("write incomplete fitting inventory");
    let proof =
        observe::telemetry::prove(&root, &command(&["observe", "prove"])).expect("prove receipt");
    assert_eq!(proof["status"], "fail");
    assert!(
        proof["why_failed"]
            .as_str()
            .unwrap()
            .contains("observability_command_inventory_missing")
    );
    assert_eq!(
        proof["next_repair"],
        "fit every law-bearing command, plugin surface, operating-loop stage, and signal inventory row, then rerun observe prove"
    );
    fs::remove_dir_all(root).expect("cleanup root");
}

fn command(raw: &[&str]) -> observe::types::ObserveCommand {
    observe::parse(&super::args(raw))
        .expect("parse")
        .expect("observe command")
}
