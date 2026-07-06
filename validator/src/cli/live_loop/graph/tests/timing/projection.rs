use super::rows::{mandatory_law_failure, node_timing};
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
    assert_eq!(fmt["claim_name"], "source-local live-loop speed claim");
    assert_eq!(fmt["claim_status"], "supported_source_local");
    assert!(
        fmt["proof_surface"]
            .as_str()
            .expect("proof surface")
            .contains("executed current-candidate command")
    );
    assert!(
        fmt["independent_reconciliation_surface"]
            .as_str()
            .expect("reconciliation surface")
            .contains("logs, metrics, traces")
    );
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
fn live_loop_tasks_project_cache_and_invalid_proof_claim_surfaces() {
    let mut cache_timings = BTreeMap::new();
    let mut cache_row = node_timing(1_000, 5, "pass".to_string(), "none".to_string());
    cache_row.proof_kind = "verified_cache_hit".to_string();
    cache_timings.insert("fmt_check".to_string(), cache_row);
    let cache_node = super::super::super::tasks(
        "sha256:candidate",
        "sha256:changed",
        "sha256:context",
        "hot",
        "verified-local",
        Some(1_000),
        &cache_timings,
    )
    .into_iter()
    .map(|task| task())
    .find(|node| node["node_id"] == "fmt_check")
    .expect("fmt node");
    assert_eq!(cache_node["claim_status"], "supported_source_local");
    assert!(
        cache_node["proof_surface"]
            .as_str()
            .expect("cache proof surface")
            .contains("verified same-candidate cache replay")
    );

    let mut invalid_row = node_timing(
        1_000,
        5,
        "fail".to_string(),
        "verified_local_proof_kind_invalid".to_string(),
    );
    invalid_row.proof_kind = "planned".to_string();
    let invalid_node = super::rows::projected_node(invalid_row);
    assert_eq!(invalid_node["status"], "blocked");
    assert_eq!(
        invalid_node["failure_class"],
        "verified_local_proof_kind_invalid"
    );
    assert_eq!(
        invalid_node["where_failed"],
        "loop.run.focused_rust_tests.proof_kind"
    );
    assert_eq!(invalid_node["claim_status"], "blocked");
    assert_eq!(
        invalid_node["proof_surface"],
        "invalid proof_kind; row is blocked"
    );
}
