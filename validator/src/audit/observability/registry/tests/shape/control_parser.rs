use std::collections::BTreeSet;

fn args(command: &str) -> Vec<String> {
    command.split_whitespace().map(ToOwned::to_owned).collect()
}

#[test]
fn production_update_goal_inventory_is_bound_to_real_control_parser() {
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

    assert!(!listed.contains("self law prove"));
    assert!(listed.contains("update-goal eligibility"));
    assert!(
        crate::cli::control::plane::parse(&args("self law prove")).is_none(),
        "phantom self law prove must not be inventoriable"
    );
    assert_eq!(
        crate::cli::control::plane::parse(&args("update-goal eligibility"))
            .expect("update-goal parses")
            .operation,
        crate::cli::control::plane::types::ControlOperation::UpdateGoalEligibility
    );
    assert_eq!(
        crate::cli::control::plane::parse(&args("self update-goal eligibility"))
            .expect("self update-goal parses")
            .operation,
        crate::cli::control::plane::types::ControlOperation::SelfUpdateGoalEligibility
    );
    assert_eq!(
        inventory
            .pointer("/command_observability_inventory/update-goal eligibility/receipt_paths/0")
            .and_then(serde_json::Value::as_str),
        Some("validation_artifacts/cli/update-goal-eligibility.json")
    );
    assert_eq!(
        inventory
            .pointer(
                "/command_observability_inventory/self update-goal eligibility/receipt_paths/0"
            )
            .and_then(serde_json::Value::as_str),
        Some("validation_artifacts/cli/self-law-receipt.json")
    );
}
