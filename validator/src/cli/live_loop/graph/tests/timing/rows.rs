use crate::cli::live_loop::nodes::command_failure::CommandFailureSummary;
use crate::cli::live_loop::nodes::timing::{NODE_TIMING_REL, NodeTiming};
use std::collections::BTreeMap;

pub(super) fn node_timing(
    baseline_duration_ms: u64,
    verified_local_duration_ms: u64,
    timing_status: String,
    failure_class: String,
) -> NodeTiming {
    NodeTiming {
        baseline_duration_ms,
        verified_local_duration_ms,
        proof_kind: "executed".to_string(),
        cache_hit: false,
        cache_key: "sha256:cache".to_string(),
        work_unit_count: 1,
        actual_work_duration_ms: verified_local_duration_ms,
        graph_overhead_ms: 1,
        equivalence_status: "executed_current_candidate_not_cache_replay".to_string(),
        invalidation_proof: "cache_not_used_current_command_executed".to_string(),
        telemetry_reconciliation_status: "pass".to_string(),
        verified_local_command: "cargo fmt --all --check".to_string(),
        verified_local_result_digest: "sha256:result".to_string(),
        verified_local_output_digest: "sha256:output".to_string(),
        where_failed: "none".to_string(),
        why_failed: "none".to_string(),
        next_repair: "none".to_string(),
        timing_status,
        failure_class,
        baseline_exit_code: Some(0),
        baseline_launch_error: false,
        baseline_failure: CommandFailureSummary::default(),
        affected_set_status: "changed_files_digest_bound".to_string(),
        timing_source: NODE_TIMING_REL.to_string(),
    }
}

pub(super) fn mandatory_law_failure() -> CommandFailureSummary {
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

pub(super) fn projected_node(timing: NodeTiming) -> serde_json::Value {
    let mut timings = BTreeMap::new();
    timings.insert("focused_rust_tests".to_string(), timing);
    super::super::super::tasks(
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
