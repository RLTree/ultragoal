use crate::cli::observe;
use serde_json::json;
use std::fs;

#[test]
fn observe_prove_reports_successor_catalog_unavailable_after_stack_passes() {
    let root = super::minimal_root("observe-prove-command-inventory-incomplete");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let health = command(&["observe", "stack", "health"]);
    let health_receipt = observe::stack::health_receipt(
        &root,
        &health,
        vec![json!({"service":"victorialogs","status":"pass"})],
    )
    .expect("health");
    crate::self_tests::boundaries::workspace_fixtures::write_json(
        &root.join(health.operation.receipt_rel()),
        &health_receipt,
    )
    .expect("write health");
    let smoke = command(&["observe", "stack", "smoke"]);
    let smoke_receipt =
        observe::stack::smoke_receipt(&root, &smoke, &candidate, true, true, true).expect("smoke");
    crate::self_tests::boundaries::workspace_fixtures::write_json(
        &root.join(smoke.operation.receipt_rel()),
        &smoke_receipt,
    )
    .expect("write smoke");
    fs::create_dir_all(root.join("docs/generated/observability")).expect("inventory parent");
    let static_inventory = root.join("docs/generated/observability/command-inventory.json");
    fs::write(&static_inventory, "SECRET_CANARY").expect("static bait");
    let proof =
        observe::telemetry::prove(&root, &command(&["observe", "prove"])).expect("prove receipt");
    fs::write(&static_inventory, [0xff, 0xfe]).expect("invalid static bait");
    let mutated =
        observe::telemetry::prove(&root, &command(&["observe", "prove"])).expect("mutated proof");
    assert_eq!(proof["status"], "fail");
    let why = proof["why_failed"].as_str().unwrap();
    assert!(why.starts_with("HCT-OBSERVE successor catalog unavailable/not adopted"));
    assert_eq!(proof["why_failed"], mutated["why_failed"]);
    assert_eq!(proof["claim_ceiling"], mutated["claim_ceiling"]);
    assert!(!why.contains("SECRET_CANARY"));
    assert_eq!(
        proof["next_repair"],
        "repair the first_failure and control_board_first_incomplete named in why_failed, then rerun observe prove"
    );
    fs::remove_dir_all(root).expect("cleanup root");
}

#[test]
fn observe_receipt_blocks_completion_when_live_stack_is_not_proven() {
    let root = super::minimal_root("observe-receipt");
    let command = command(&["observe", "prove"]);
    let receipt = observe::telemetry::prove(&root, &command).expect("receipt");
    assert_eq!(receipt["status"], "fail");
    assert_eq!(
        receipt["claim_ceiling"],
        "observability_product_closure_failed_completion_readiness_release_update_goal_blocked"
    );
    assert!(receipt["event"]["duration_ms"].as_u64().unwrap() > 0);
    assert_eq!(receipt["event"]["worker_count"], 1);
    assert_eq!(receipt["event"]["task_count"], 1);
    assert_eq!(receipt["event"]["queue_depth"], 0);
    assert_eq!(receipt["event"]["cache_mode"], "observe_command_no_cache");
    assert_eq!(
        receipt["event"]["resource_measurement_status"],
        "wall_time_only_cpu_memory_io_unavailable"
    );
    assert_eq!(
        receipt["metric"]["duration_ms"],
        receipt["event"]["duration_ms"]
    );
    assert_eq!(
        receipt["trace"]["worker_count"],
        receipt["event"]["worker_count"]
    );
    assert!(
        receipt["query_examples"][0]
            .as_str()
            .unwrap()
            .contains(receipt["run_id"].as_str().unwrap())
    );
    assert!(
        receipt["blocked_claims"]
            .as_array()
            .expect("blocked")
            .iter()
            .any(|claim| claim.as_str() == Some("update_goal_eligibility"))
    );
    assert!(
        root.join("validation_artifacts/observability/spool/events.jsonl")
            .is_file()
    );
    fs::remove_dir_all(root).expect("cleanup observe receipt");
}

fn command(raw: &[&str]) -> observe::command::ObserveCommand {
    observe::parse(&super::args(raw))
        .expect("parse")
        .expect("observe command")
}
