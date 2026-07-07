use super::{LiveLoopAction, LiveLoopCommand, run};
use serde_json::json;

#[test]
fn live_loop_run_refreshes_node_timing_or_stays_on_reconciliation_failure() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-pass");
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    crate::json_boundary::write_json(
        &root.join("docs/generated/observability/command-inventory.json"),
        &json!({"observability_control_board": {"status": "observable"}}),
    )
    .expect("board");
    for path in [
        "validation_artifacts/coverage/coverage-receipt.json",
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
        "validation_artifacts/ultragoal-audit/red-fixture-report.json",
    ] {
        crate::json_boundary::write_json(
            &root.join(path),
            &json!({"status": "pass", "target_digest": candidate}),
        )
        .expect("source-local receipt");
    }

    let command = LiveLoopCommand {
        action: LiveLoopAction::Run,
        tier: "hot".to_string(),
        cache_mode: "verified-local".to_string(),
        jobs: Some(2),
        receipt: "validation_artifacts/observability/loop-pass.json".into(),
        node_id: None,
        measure_all: false,
    };
    let code = run(&root, &command).expect("loop blocked run");
    assert_eq!(code, 1);
    let receipt = crate::json_boundary::read_json(&root.join(&command.receipt)).expect("receipt");
    assert_eq!(receipt["status"], "fail");
    let timing_refreshes = receipt["timing_refreshes"]
        .as_array()
        .expect("timing refreshes");
    let refreshed_ids: Vec<&str> = timing_refreshes
        .iter()
        .map(|refresh| refresh["node_id"].as_str().expect("refresh node id"))
        .collect();
    assert_eq!(refreshed_ids, ["fmt_check"]);
    assert!(timing_refreshes.iter().all(|refresh| {
        refresh["task_class"] == "shared_authority_write_serial"
            && refresh["status"] == "measurement_batch_result_recorded"
            && refresh.get("exit_code").is_none()
            && refresh["exit_code_scope"] == "aggregate_for_refresh_batch_not_per_node"
            && refresh["refresh_batch_exit_code"].as_i64().is_some()
    }));
    assert_eq!(
        receipt["audit_context"]["changed_inputs"]["affected_node_count"],
        0
    );
    assert!(
        receipt["audit_context"]["changed_inputs"]["unaffected_node_count"]
            .as_u64()
            .expect("unaffected node count")
            > 0
    );
    let fmt_node = receipt["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|node| node["node_id"] == "fmt_check")
        .expect("fmt node");
    assert_eq!(fmt_node["validation_status"], "pass");
    assert_eq!(fmt_node["validation_cache_status"], "reusable");
    assert_ne!(fmt_node["status"], "fail");
    let build_node = receipt["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|node| node["node_id"] == "build_check")
        .expect("build node");
    match (
        receipt["first_blocker"]["id"].as_str().unwrap_or(""),
        receipt["first_blocker"]["failure_class"]
            .as_str()
            .unwrap_or(""),
    ) {
        ("fmt_check", "live_loop_speedup_target_missed") => {
            assert_executed_timing_refresh_blocked(fmt_node, "cargo fmt");
        }
        ("build_check", "live_loop_speedup_target_missed") => {
            assert_executed_timing_refresh_blocked(build_node, "cargo build");
        }
        ("build_check", "live_loop_high_frequency_measurement_missing") => {
            assert_eq!(
                build_node["baseline_measurement_state"],
                "missing_current_full_command_baseline"
            );
            assert_eq!(build_node["proof_kind"], "missing");
        }
        ("fmt_check", "live_loop_telemetry_reconciliation_missing") => {
            assert_reconciliation_failed(fmt_node);
        }
        (
            "line_caps_check" | "namespace_check" | "schema_validation" | "package_inventory",
            "canonical_full_command_failed",
        ) => {
            assert_temp_root_cli_node_failed_with_agent_legible_repair(&receipt);
        }
        other => panic!("unexpected live-loop blocker state: {other:?}"),
    }
    assert!(
        receipt["first_product_blocker"]["id"] == receipt["first_blocker"]["id"]
            || receipt["first_product_blocker"]["id"] == "none",
        "{}",
        receipt["first_product_blocker"]
    );
    assert!(
        receipt["first_speed_blocker"]["id"].as_str().is_some(),
        "{}",
        receipt["first_speed_blocker"]
    );
    assert!(
        receipt["first_observability_blocker"]["id"]
            .as_str()
            .is_some(),
        "{}",
        receipt["first_observability_blocker"]
    );
    assert!(
        receipt["first_control_board_blocker"]["id"]
            .as_str()
            .is_some(),
        "{}",
        receipt["first_control_board_blocker"]
    );
    assert_eq!(
        receipt["observability"]["event"]["failure_class"],
        receipt["first_blocker"]["failure_class"]
    );
    std::fs::remove_dir_all(root).expect("cleanup live loop pass");
}

fn assert_executed_timing_refresh_blocked(node: &serde_json::Value, command_fragment: &str) {
    assert_eq!(
        node["baseline_measurement_state"],
        "current_full_command_baseline_observed"
    );
    assert_eq!(
        node["speedup_measurement_state"],
        "verified_local_20x_proof_failed"
    );
    assert_eq!(node["proof_kind"], "executed");
    assert_eq!(node["cache_hit"], false);
    assert_eq!(node["work_unit_count"], 1);
    assert_eq!(node["telemetry_reconciliation_status"], "pass");
    let next_repair = node["next_repair"].as_str().expect("next repair text");
    assert!(
        next_repair.contains("split, cache")
            && next_repair.contains(command_fragment)
            && next_repair.contains("verified equivalence"),
        "{next_repair}"
    );
}

fn assert_reconciliation_failed(fmt_node: &serde_json::Value) {
    assert_eq!(
        fmt_node["failure_class"],
        "live_loop_telemetry_reconciliation_missing"
    );
    assert_eq!(
        fmt_node["telemetry_reconciliation_status"],
        "query_or_explain_reconciliation_failed"
    );
}

fn assert_temp_root_cli_node_failed_with_agent_legible_repair(receipt: &serde_json::Value) {
    let blocker = &receipt["first_blocker"];
    assert_eq!(blocker["failure_class"], "canonical_full_command_failed");
    let next_repair = blocker["next_repair"].as_str().expect("next repair");
    assert!(next_repair.contains("run `target/debug/ultragoal --root ."));
    assert!(next_repair.contains("repair the command behavior"));
    assert!(next_repair.contains("loop measure --node"));
}
