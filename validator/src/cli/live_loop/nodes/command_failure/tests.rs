use super::CommandFailureSummary;
use serde_json::json;

#[test]
fn command_failure_summary_extracts_agent_actionable_stdout_fields() {
    let stdout = b"ultragoal-mandatory-law-validation fail candidate=sha256:current \
failed_law=full-local-observability-stack-integration-non-opaque-failure \
failed_check=mandatory-law-validation-observability-binding \
why=mandatory law validation failed: stale receipt where=mandatory-law.validation \
claim_impact=blocks_readiness next_repair=repair stale receipt, then rerun mandatory-law validation \
receipt=validation_artifacts/observability/mandatory-law-validation.json run_id=run-123 \
correlation_id=corr-456 query_logs='ultragoal observe logs query --run-id run-123' \
query_metrics='ultragoal observe metrics query --query \"bounded\" --limit 100' \
query_traces='ultragoal observe traces query --run-id run-123'";
    let summary = CommandFailureSummary::from_stdout(stdout);
    assert_eq!(
        summary.failed_law.as_deref(),
        Some("full-local-observability-stack-integration-non-opaque-failure")
    );
    assert_eq!(
        summary.where_failed.as_deref(),
        Some("mandatory-law.validation")
    );
    assert!(
        summary
            .why_failed
            .as_deref()
            .expect("why")
            .contains("stale receipt")
    );
    assert!(
        summary
            .query_metrics
            .as_deref()
            .expect("metrics")
            .contains("--limit 100")
    );
    assert!(summary.why_failed.is_some());
    assert!(summary.next_repair.is_some());
}

#[test]
fn command_failure_summary_uses_failure_detail_line_for_repair_ids() {
    let stdout = b"ultragoal-mandatory-law-validation fail operation=mandatory-law.validation candidate=sha256:current receipt=validation_artifacts/observability/mandatory-law-validation.json run_id=run-summary correlation_id=corr-summary claim_impact=mandatory_law_validation_failed_blocks_readiness_release_completion_update_goal supported_claims=none unsupported_claims=completion,readiness\n\
failed_law=full-local-observability-stack-integration-non-opaque-failure failed_check=mandatory-law-validation-observability-binding why=mandatory law validation failed: stale evidence where=mandatory-law.validation claim_impact=mandatory_law_validation_failed_blocks_readiness_release_completion_update_goal next_repair=query this run through observe logs/metrics/traces, repair stale evidence, then rerun mandatory-law validation receipt=validation_artifacts/observability/mandatory-law-validation.json run_id=run-detail correlation_id=corr-detail query_logs='ultragoal observe logs query --run-id run-detail --limit 100' query_metrics='ultragoal observe metrics query --query bounded --limit 100' query_traces='ultragoal observe traces query --run-id run-detail --limit 100'";
    let summary = CommandFailureSummary::from_stdout(stdout);
    assert_eq!(summary.run_id.as_deref(), Some("run-detail"));
    assert_eq!(summary.correlation_id.as_deref(), Some("corr-detail"));
    assert_eq!(
        summary.claim_impact.as_deref(),
        Some("mandatory_law_validation_failed_blocks_readiness_release_completion_update_goal")
    );
    assert!(
        summary
            .query_logs
            .as_deref()
            .expect("query logs")
            .contains("run-detail")
    );
    assert!(
        !summary
            .correlation_id
            .as_deref()
            .expect("correlation")
            .contains("claim_impact"),
        "{summary:?}"
    );
}

#[test]
fn command_failure_summary_reads_bounded_receipt_fields() {
    let long_next_repair = "repair-source ".repeat(200);
    let value = json!({
        "failed_law": "full-local-observability-stack-integration-non-opaque-failure",
        "failed_check": "mandatory-law-validation-observability-binding",
        "why_failed": "mandatory law validation failed\nbecause receipt is stale",
        "where_failed": "mandatory-law.validation",
        "claim_impact": "blocks_readiness",
        "next_repair": long_next_repair,
        "receipt": "validation_artifacts/observability/mandatory-law-validation.json",
        "run_id": "run-123",
        "correlation_id": "corr-456",
        "query_logs": "ultragoal observe logs query --run-id run-123",
        "query_metrics": "ultragoal observe metrics query --query bounded",
        "query_traces": "ultragoal observe traces query --run-id run-123"
    });
    let summary = CommandFailureSummary::from_value(Some(&value));
    assert_eq!(
        summary.failed_check.as_deref(),
        Some("mandatory-law-validation-observability-binding")
    );
    assert_eq!(
        summary.why_failed.as_deref(),
        Some("mandatory law validation failed because receipt is stale")
    );
    assert!(
        summary
            .next_repair
            .as_deref()
            .expect("repair")
            .ends_with("...")
    );
    assert_eq!(summary.run_id.as_deref(), Some("run-123"));
    assert_eq!(summary.correlation_id.as_deref(), Some("corr-456"));
    assert_eq!(
        summary.query_traces.as_deref(),
        Some("ultragoal observe traces query --run-id run-123")
    );
}

#[test]
fn command_failure_summary_ignores_absent_and_empty_receipt_fields() {
    assert!(CommandFailureSummary::from_value(None).why_failed.is_none());
    let value = json!({
        "why_failed": "   ",
        "where_failed": "\n",
        "run_id": 17
    });
    let summary = CommandFailureSummary::from_value(Some(&value));
    assert!(summary.why_failed.is_none());
    assert!(summary.where_failed.is_none());
    assert!(summary.run_id.is_none());
}
