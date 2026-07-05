use super::rows::{mandatory_law_failure, node_timing, projected_node};
use crate::cli::live_loop::nodes::command_failure::CommandFailureSummary;
use crate::cli::live_loop::nodes::timing::{NODE_TIMING_REL, NodeTiming};
use std::collections::BTreeMap;

#[test]
fn live_loop_tasks_project_current_node_timing_records() {
    let mut timings = BTreeMap::new();
    timings.insert(
        "fmt_check".to_string(),
        node_timing(1_000, 5, "pass".to_string(), "none".to_string()),
    );
    let nodes = super::super::super::tasks(
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
    assert_eq!(fmt["timing_source"], NODE_TIMING_REL);
    assert_eq!(fmt["graph_task_class"], "pure_read_parallel");
    assert_eq!(fmt["execution_task_class"], "pure_read_parallel");
    assert_eq!(fmt["execution_serial_reason"], "none");
    assert_eq!(fmt["baseline_duration_ms"], 1_000);
    assert_eq!(fmt["verified_local_duration_ms"], 5);
    assert_eq!(fmt["proof_kind"], "executed");
    assert_eq!(fmt["work_unit_count"], 1);
    assert_eq!(fmt["actual_work_duration_ms"], 5);
    assert_eq!(fmt["telemetry_reconciliation_status"], "pass");
    assert!(
        fmt["result_digest"]
            .as_str()
            .expect("result digest")
            .starts_with("sha256:")
    );
    assert_eq!(fmt["result_digest"], fmt["verified_local_result_digest"]);
    assert!(
        fmt["verified_local_result_digest"]
            .as_str()
            .expect("result digest")
            .starts_with("sha256:")
    );
}

#[test]
fn live_loop_tasks_project_failed_full_command_timing_rows() {
    let mut timings = BTreeMap::new();
    timings.insert(
        "focused_rust_tests".to_string(),
        NodeTiming {
            baseline_duration_ms: 100_000,
            verified_local_duration_ms: 1,
            proof_kind: "executed".to_string(),
            cache_hit: false,
            cache_key: "sha256:cache".to_string(),
            work_unit_count: 1,
            actual_work_duration_ms: 1,
            graph_overhead_ms: 1,
            equivalence_status: "executed_current_candidate_not_cache_replay".to_string(),
            invalidation_proof: "cache_not_used_current_command_executed".to_string(),
            telemetry_reconciliation_status: "pass".to_string(),
            verified_local_command: "cargo test --offline live_loop --lib --quiet".to_string(),
            result_digest: "sha256:result".to_string(),
            output_digest: "sha256:output".to_string(),
            verified_local_result_digest: "sha256:result".to_string(),
            verified_local_output_digest: "sha256:output".to_string(),
            where_failed: "mandatory-law.validation".to_string(),
            why_failed: "mandatory law validation failed: stale receipt".to_string(),
            next_repair: "query this run through observe logs/metrics/traces, repair the named mandatory-law row, fixture, dependency, or evidence digest, then rerun mandatory-law validation".to_string(),
            timing_status: "fail".to_string(),
            failure_class: "canonical_full_command_failed".to_string(),
            baseline_exit_code: Some(101),
            baseline_launch_error: false,
            baseline_failure: mandatory_law_failure(),
            affected_set_status: "clean_worktree_no_affected_files".to_string(),
            timing_source: NODE_TIMING_REL.to_string(),
        },
    );
    let nodes = super::super::super::tasks(
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
        proof_kind: "executed".to_string(),
        cache_hit: false,
        cache_key: "sha256:cache".to_string(),
        work_unit_count: 1,
        actual_work_duration_ms: 1,
        graph_overhead_ms: 1,
        equivalence_status: "executed_current_candidate_not_cache_replay".to_string(),
        invalidation_proof: "cache_not_used_current_command_executed".to_string(),
        telemetry_reconciliation_status: "pass".to_string(),
        verified_local_command: "cargo test --offline live_loop --lib --quiet".to_string(),
        result_digest: "sha256:result".to_string(),
        output_digest: "sha256:output".to_string(),
        verified_local_result_digest: "sha256:result".to_string(),
        verified_local_output_digest: "sha256:output".to_string(),
        where_failed: "loop.measure.focused_rust_tests.canonical_full_command".to_string(),
        why_failed: "canonical full command could not launch while measuring live-loop node"
            .to_string(),
        next_repair: "run cargo test directly and repair launch failure".to_string(),
        timing_status: "fail".to_string(),
        failure_class: "canonical_full_command_launch_failed".to_string(),
        baseline_exit_code: None,
        baseline_launch_error: true,
        baseline_failure: CommandFailureSummary::default(),
        affected_set_status: "changed_files_digest_bound".to_string(),
        timing_source: NODE_TIMING_REL.to_string(),
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
        proof_kind: "executed".to_string(),
        cache_hit: false,
        cache_key: "sha256:cache".to_string(),
        work_unit_count: 1,
        actual_work_duration_ms: 99_000,
        graph_overhead_ms: 1,
        equivalence_status: "executed_current_candidate_not_cache_replay".to_string(),
        invalidation_proof: "cache_not_used_current_command_executed".to_string(),
        telemetry_reconciliation_status: "pass".to_string(),
        verified_local_command: "cargo test --offline live_loop --lib --quiet".to_string(),
        result_digest: "sha256:result".to_string(),
        output_digest: "sha256:output".to_string(),
        verified_local_result_digest: "sha256:result".to_string(),
        verified_local_output_digest: "sha256:output".to_string(),
        where_failed: "loop.measure.focused_rust_tests.speedup".to_string(),
        why_failed: "executed verified-local work did not meet the 20x speed target".to_string(),
        next_repair: "split or cache focused_rust_tests with verified equivalence".to_string(),
        timing_status: "fail".to_string(),
        failure_class: "live_loop_speedup_target_missed".to_string(),
        baseline_exit_code: Some(0),
        baseline_launch_error: false,
        baseline_failure: CommandFailureSummary::default(),
        affected_set_status: "changed_files_digest_bound".to_string(),
        timing_source: NODE_TIMING_REL.to_string(),
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
        proof_kind: "executed".to_string(),
        cache_hit: false,
        cache_key: "sha256:cache".to_string(),
        work_unit_count: 1,
        actual_work_duration_ms: 1,
        graph_overhead_ms: 1,
        equivalence_status: "executed_current_candidate_not_cache_replay".to_string(),
        invalidation_proof: "cache_not_used_current_command_executed".to_string(),
        telemetry_reconciliation_status: "pass".to_string(),
        verified_local_command: "cargo test --offline live_loop --lib --quiet".to_string(),
        result_digest: "sha256:result".to_string(),
        output_digest: "sha256:output".to_string(),
        verified_local_result_digest: "sha256:result".to_string(),
        verified_local_output_digest: "sha256:output".to_string(),
        where_failed: "loop.measure.focused_rust_tests.measurement".to_string(),
        why_failed: "live-loop node timing row failed strict proof validation".to_string(),
        next_repair: "inspect live-loop timing receipt fields".to_string(),
        timing_status: "fail".to_string(),
        failure_class: "unexpected_measurement_failure".to_string(),
        baseline_exit_code: Some(2),
        baseline_launch_error: false,
        baseline_failure: CommandFailureSummary::default(),
        affected_set_status: "changed_files_digest_bound".to_string(),
        timing_source: NODE_TIMING_REL.to_string(),
    });
    assert_eq!(
        unknown["failure_class"],
        "live_loop_node_measurement_failed"
    );
    assert!(
        unknown["why_failed"]
            .as_str()
            .expect("why")
            .contains("strict proof validation")
    );
}
