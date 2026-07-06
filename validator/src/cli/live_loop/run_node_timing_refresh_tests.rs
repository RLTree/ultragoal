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
    assert_eq!(timing_refreshes[0]["node_id"], "fmt_check");
    assert_eq!(
        timing_refreshes[0]["task_class"],
        "shared_authority_write_serial"
    );
    let fmt_node = receipt["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|node| node["node_id"] == "fmt_check")
        .expect("fmt node");
    match (
        receipt["first_blocker"]["id"].as_str().unwrap_or(""),
        receipt["first_blocker"]["failure_class"]
            .as_str()
            .unwrap_or(""),
    ) {
        ("fmt_check", "live_loop_speedup_target_missed") => {
            assert_executed_timing_refresh_blocked(timing_refreshes, fmt_node);
        }
        ("fmt_check", "live_loop_telemetry_reconciliation_missing") => {
            assert_reconciliation_failed(timing_refreshes, fmt_node);
        }
        other => panic!("unexpected live-loop blocker state: {other:?}"),
    }
    assert_eq!(
        receipt["observability"]["event"]["failure_class"],
        "observability_live_loop_first_blocker"
    );
    std::fs::remove_dir_all(root).expect("cleanup live loop pass");
}

fn assert_executed_timing_refresh_blocked(
    timing_refreshes: &[serde_json::Value],
    fmt_node: &serde_json::Value,
) {
    assert_eq!(timing_refreshes.len(), 1);
    assert_eq!(
        timing_refreshes[0]["status"],
        "measurement_command_result_recorded"
    );
    assert_eq!(
        fmt_node["baseline_measurement_state"],
        "current_full_command_baseline_observed"
    );
    assert_eq!(
        fmt_node["speedup_measurement_state"],
        "verified_local_20x_proof_failed"
    );
    assert_eq!(fmt_node["proof_kind"], "executed");
    assert_eq!(fmt_node["cache_hit"], false);
    assert_eq!(fmt_node["work_unit_count"], 1);
    assert_eq!(fmt_node["telemetry_reconciliation_status"], "pass");
    let next_repair = fmt_node["next_repair"].as_str().expect("next repair text");
    assert!(
        next_repair.contains("split or cache `cargo fmt --all --check` with verified equivalence"),
        "{next_repair}"
    );
    assert!(
        next_repair.contains(
            "rerun `target/debug/ultragoal --root . loop measure --node fmt_check --tier hot --cache-mode verified-local`"
        ),
        "{next_repair}"
    );
}

fn assert_reconciliation_failed(
    timing_refreshes: &[serde_json::Value],
    fmt_node: &serde_json::Value,
) {
    assert_eq!(timing_refreshes.len(), 1);
    assert_eq!(
        timing_refreshes[0]["status"],
        "measurement_command_result_recorded"
    );
    assert_eq!(
        fmt_node["failure_class"],
        "live_loop_telemetry_reconciliation_missing"
    );
    assert_eq!(
        fmt_node["telemetry_reconciliation_status"],
        "query_or_explain_reconciliation_failed"
    );
}
