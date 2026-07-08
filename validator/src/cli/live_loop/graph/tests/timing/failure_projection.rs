use super::rows::projected_node;
use crate::cli::live_loop::nodes::command_failure::CommandFailureSummary;
use crate::cli::live_loop::nodes::timing::{NODE_TIMING_REL, NodeTiming};
use serde_json::json;

use super::rows::node_timing;
use crate::cli::live_loop::changed_inputs::ChangedInputs;
use std::collections::BTreeMap;

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
        reconciled_command_duration_ms: 3,
        product_latency_ms: 2,
        equivalence_status: "executed_current_candidate_not_cache_replay".to_string(),
        invalidation_proof: "cache_not_used_current_command_executed".to_string(),
        telemetry_reconciliation_status: "pass".to_string(),
        validation_status: "pass".to_string(),
        validation_cache_status: "reusable".to_string(),
        observability_status: "pass".to_string(),
        speed_claim_status: "failed".to_string(),
        observability_failure_class: "none".to_string(),
        verified_local_command: "cargo test --offline live_loop::nodes::measurement --lib --quiet"
            .to_string(),
        result_digest: "sha256:result".to_string(),
        output_digest: "sha256:output".to_string(),
        verified_local_result_digest: "sha256:result".to_string(),
        verified_local_output_digest: "sha256:output".to_string(),
        where_failed: "loop.measure.live_loop_measurement_rust_tests.canonical_full_command"
            .to_string(),
        why_failed: "canonical full command could not launch while measuring live-loop node"
            .to_string(),
        next_repair: "run cargo test directly and repair launch failure".to_string(),
        timing_status: "fail".to_string(),
        failure_class: "canonical_full_command_launch_failed".to_string(),
        baseline_proof_kind: "executed".to_string(),
        baseline_invalidation_proof: "baseline_command_executed_for_current_measurement"
            .to_string(),
        baseline_exit_code: None,
        baseline_launch_error: true,
        baseline_failure: CommandFailureSummary::default(),
        telemetry_reconciliation: json!({"status": "pass"}).into(),
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
        reconciled_command_duration_ms: 99_002,
        product_latency_ms: 99_001,
        equivalence_status: "executed_current_candidate_not_cache_replay".to_string(),
        invalidation_proof: "cache_not_used_current_command_executed".to_string(),
        telemetry_reconciliation_status: "pass".to_string(),
        validation_status: "pass".to_string(),
        validation_cache_status: "reusable".to_string(),
        observability_status: "pass".to_string(),
        speed_claim_status: "failed".to_string(),
        observability_failure_class: "none".to_string(),
        verified_local_command: "cargo test --offline live_loop::nodes::measurement --lib --quiet"
            .to_string(),
        result_digest: "sha256:result".to_string(),
        output_digest: "sha256:output".to_string(),
        verified_local_result_digest: "sha256:result".to_string(),
        verified_local_output_digest: "sha256:output".to_string(),
        where_failed: "loop.measure.live_loop_measurement_rust_tests.speedup".to_string(),
        why_failed: "executed verified-local work did not meet the 20x speed target".to_string(),
        next_repair: "split or cache live_loop_measurement_rust_tests with verified equivalence"
            .to_string(),
        timing_status: "fail".to_string(),
        failure_class: "live_loop_speedup_target_missed".to_string(),
        baseline_proof_kind: "executed".to_string(),
        baseline_invalidation_proof: "baseline_command_executed_for_current_measurement"
            .to_string(),
        baseline_exit_code: Some(0),
        baseline_launch_error: false,
        baseline_failure: CommandFailureSummary::default(),
        telemetry_reconciliation: json!({"status": "pass"}).into(),
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
        reconciled_command_duration_ms: 3,
        product_latency_ms: 2,
        equivalence_status: "executed_current_candidate_not_cache_replay".to_string(),
        invalidation_proof: "cache_not_used_current_command_executed".to_string(),
        telemetry_reconciliation_status: "pass".to_string(),
        validation_status: "pass".to_string(),
        validation_cache_status: "reusable".to_string(),
        observability_status: "pass".to_string(),
        speed_claim_status: "failed".to_string(),
        observability_failure_class: "none".to_string(),
        verified_local_command: "cargo test --offline live_loop::nodes::measurement --lib --quiet"
            .to_string(),
        result_digest: "sha256:result".to_string(),
        output_digest: "sha256:output".to_string(),
        verified_local_result_digest: "sha256:result".to_string(),
        verified_local_output_digest: "sha256:output".to_string(),
        where_failed: "loop.measure.live_loop_measurement_rust_tests.measurement".to_string(),
        why_failed: "live-loop node timing row failed strict proof validation".to_string(),
        next_repair: "inspect live-loop timing receipt fields".to_string(),
        timing_status: "fail".to_string(),
        failure_class: "unexpected_measurement_failure".to_string(),
        baseline_proof_kind: "executed".to_string(),
        baseline_invalidation_proof: "baseline_command_executed_for_current_measurement"
            .to_string(),
        baseline_exit_code: Some(2),
        baseline_launch_error: false,
        baseline_failure: CommandFailureSummary::default(),
        telemetry_reconciliation: json!({"status": "pass"}).into(),
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

#[test]
fn live_loop_tasks_project_context_measurements_as_observations() {
    let mut package_digest = node_timing(100, 10, "partial".to_string(), "none".to_string());
    package_digest.speed_claim_status = "withheld".to_string();
    let mut timings = BTreeMap::new();
    timings.insert("package_digest".to_string(), package_digest);

    let node = super::super::super::tasks(
        "sha256:candidate",
        "sha256:changed",
        "sha256:context",
        "hot",
        "verified-local",
        Some(100),
        &timings,
        &ChangedInputs::for_tests("sha256:changed", "sha256:context"),
    )
    .into_iter()
    .map(|task| task())
    .find(|node| node["node_id"] == "package_digest")
    .expect("package digest node");

    assert_eq!(node["status"], "observed");
    assert_eq!(node["failure_class"], "none");
    assert_eq!(node["validation_status"], "pass");
    assert_eq!(node["observability_status"], "pass");
    assert_eq!(node["speed_claim_status"], "withheld");
    assert_eq!(
        node["claim_impact"],
        "observation_only_no_speed_readiness_release_completion_or_update_goal_claim"
    );
}

#[test]
fn live_loop_tasks_accept_legacy_context_speed_rows_without_speed_claim() {
    let package_digest = node_timing(100, 10, "pass".to_string(), "none".to_string());
    let mut timings = BTreeMap::new();
    timings.insert("package_digest".to_string(), package_digest);

    let node = super::super::super::tasks(
        "sha256:candidate",
        "sha256:changed",
        "sha256:context",
        "hot",
        "verified-local",
        Some(100),
        &timings,
        &ChangedInputs::for_tests("sha256:changed", "sha256:context"),
    )
    .into_iter()
    .map(|task| task())
    .find(|node| node["node_id"] == "package_digest")
    .expect("package digest node");

    assert_eq!(node["status"], "observed");
    assert_eq!(node["failure_class"], "none");
    assert_eq!(node["validation_status"], "pass");
    assert_eq!(node["observability_status"], "pass");
    assert_eq!(node["speed_claim_status"], "withheld");
    assert_eq!(node["claim_status"], "observation_only");
}
