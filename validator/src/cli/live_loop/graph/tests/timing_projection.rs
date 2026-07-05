use super::super::super::nodes::command_failure::CommandFailureSummary;
use super::super::super::nodes::timing::NodeTiming;
use std::collections::BTreeMap;

#[test]
fn live_loop_tasks_project_current_node_timing_records() {
    let mut timings = BTreeMap::new();
    timings.insert(
        "fmt_check".to_string(),
        NodeTiming {
            baseline_duration_ms: 1_000,
            verified_local_duration_ms: 5,
            timing_status: "pass".to_string(),
            failure_class: "none".to_string(),
            baseline_exit_code: Some(0),
            baseline_launch_error: false,
            baseline_failure: CommandFailureSummary::default(),
            affected_set_status: "changed_files_digest_bound".to_string(),
            timing_source: super::super::super::nodes::timing::NODE_TIMING_REL.to_string(),
        },
    );
    let nodes = super::super::tasks(
        "sha256:candidate",
        "sha256:changed",
        "sha256:context",
        "hot",
        "verified-local",
        Some(1_000),
        &timings,
    )
    .into_iter()
    .map(|task| task())
    .collect::<Vec<_>>();
    let fmt = nodes
        .iter()
        .find(|node| node["node_id"] == "fmt_check")
        .expect("fmt node");
    assert_eq!(fmt["status"], "pass");
    assert_eq!(fmt["affected_set_status"], "changed_files_digest_bound");
    assert_eq!(
        fmt["timing_source"],
        super::super::super::nodes::timing::NODE_TIMING_REL
    );
    assert_eq!(fmt["graph_task_class"], "pure_read_parallel");
    assert_eq!(fmt["execution_task_class"], "pure_read_parallel");
    assert_eq!(fmt["execution_serial_reason"], "none");
    assert_eq!(fmt["baseline_duration_ms"], 1_000);
    assert_eq!(fmt["verified_local_duration_ms"], 5);
}

#[test]
fn live_loop_tasks_project_failed_full_command_timing_rows() {
    let mut timings = BTreeMap::new();
    timings.insert(
        "focused_rust_tests".to_string(),
        NodeTiming {
            baseline_duration_ms: 100_000,
            verified_local_duration_ms: 1,
            timing_status: "fail".to_string(),
            failure_class: "canonical_full_command_failed".to_string(),
            baseline_exit_code: Some(101),
            baseline_launch_error: false,
            baseline_failure: mandatory_law_failure(),
            affected_set_status: "clean_worktree_no_affected_files".to_string(),
            timing_source: super::super::super::nodes::timing::NODE_TIMING_REL.to_string(),
        },
    );
    let nodes = super::super::tasks(
        "sha256:candidate",
        "sha256:changed",
        "sha256:context",
        "hot",
        "verified-local",
        Some(1_000),
        &timings,
    )
    .into_iter()
    .map(|task| task())
    .collect::<Vec<_>>();
    let focused = nodes
        .iter()
        .find(|node| node["node_id"] == "focused_rust_tests")
        .expect("focused rust node");
    assert_eq!(focused["status"], "blocked");
    assert_eq!(focused["failure_class"], "canonical_full_command_failed");
    assert_eq!(focused["baseline_exit_code"], 101);
    assert_eq!(
        focused["baseline_measurement_state"],
        "current_full_command_baseline_failed"
    );
    assert!(
        focused["next_repair"]
            .as_str()
            .expect("repair")
            .contains("repair the named mandatory-law row")
    );
    assert_eq!(focused["where_failed"], "mandatory-law.validation");
    assert_eq!(
        focused["baseline_failed_law"],
        "full-local-observability-stack-integration-non-opaque-failure"
    );
}

#[test]
fn live_loop_tasks_project_launch_speedup_and_unknown_timing_failures() {
    let launch = projected_node(NodeTiming {
        baseline_duration_ms: 100_000,
        verified_local_duration_ms: 1,
        timing_status: "fail".to_string(),
        failure_class: "canonical_full_command_launch_failed".to_string(),
        baseline_exit_code: None,
        baseline_launch_error: true,
        baseline_failure: CommandFailureSummary::default(),
        affected_set_status: "changed_files_digest_bound".to_string(),
        timing_source: super::super::super::nodes::timing::NODE_TIMING_REL.to_string(),
    });
    assert_eq!(
        launch["failure_class"],
        "canonical_full_command_launch_failed"
    );
    assert_eq!(
        launch["baseline_measurement_state"],
        "current_full_command_launch_failed"
    );
    assert!(
        launch["why_failed"]
            .as_str()
            .expect("why")
            .contains("could not launch")
    );

    let slow = projected_node(NodeTiming {
        baseline_duration_ms: 100_000,
        verified_local_duration_ms: 99_000,
        timing_status: "fail".to_string(),
        failure_class: "live_loop_speedup_target_missed".to_string(),
        baseline_exit_code: Some(0),
        baseline_launch_error: false,
        baseline_failure: CommandFailureSummary::default(),
        affected_set_status: "changed_files_digest_bound".to_string(),
        timing_source: super::super::super::nodes::timing::NODE_TIMING_REL.to_string(),
    });
    assert_eq!(slow["failure_class"], "live_loop_speedup_target_missed");
    assert!(
        slow["why_failed"]
            .as_str()
            .expect("why")
            .contains("20x speed target")
    );

    let unknown = projected_node(NodeTiming {
        baseline_duration_ms: 100_000,
        verified_local_duration_ms: 1,
        timing_status: "fail".to_string(),
        failure_class: "unexpected_measurement_failure".to_string(),
        baseline_exit_code: Some(2),
        baseline_launch_error: false,
        baseline_failure: CommandFailureSummary::default(),
        affected_set_status: "changed_files_digest_bound".to_string(),
        timing_source: super::super::super::nodes::timing::NODE_TIMING_REL.to_string(),
    });
    assert_eq!(
        unknown["failure_class"],
        "live_loop_node_measurement_failed"
    );
    assert!(
        unknown["why_failed"]
            .as_str()
            .expect("why")
            .contains("timing row is failed")
    );
}

fn mandatory_law_failure() -> CommandFailureSummary {
    CommandFailureSummary {
        failed_law: Some("full-local-observability-stack-integration-non-opaque-failure".into()),
        failed_check: Some("mandatory-law-validation-observability-binding".into()),
        why_failed: Some("mandatory law validation failed: stale receipt".into()),
        where_failed: Some("mandatory-law.validation".into()),
        claim_impact: Some("mandatory_law_validation_failed_blocks_readiness_release_completion_update_goal".into()),
        next_repair: Some("query this run through observe logs/metrics/traces, repair the named mandatory-law row, fixture, dependency, or evidence digest, then rerun mandatory-law validation".into()),
        receipt: Some("validation_artifacts/observability/mandatory-law-validation.json".into()),
        run_id: Some("run-mandatory-law".into()),
        correlation_id: Some("corr-mandatory-law".into()),
        query_logs: None,
        query_metrics: None,
        query_traces: None,
    }
}

fn projected_node(timing: NodeTiming) -> serde_json::Value {
    let mut timings = BTreeMap::new();
    timings.insert("focused_rust_tests".to_string(), timing);
    super::super::tasks(
        "sha256:candidate",
        "sha256:changed",
        "sha256:context",
        "hot",
        "verified-local",
        Some(1_000),
        &timings,
    )
    .into_iter()
    .map(|task| task())
    .find(|node| node["node_id"] == "focused_rust_tests")
    .expect("focused rust node")
}
