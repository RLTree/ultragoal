use serde_json::json;
use std::fs;

pub(super) fn write_run_event(root: &std::path::Path, run_id: &str, why_failed: &str) {
    let path = root.join("validation_artifacts/observability/spool/events.jsonl");
    fs::create_dir_all(path.parent().unwrap()).expect("spool dir");
    let candidate = crate::package::inventory::package_digest(root).expect("digest");
    fs::write(
        path,
        format!(
            "{}\n",
            json!({
                "run_id": run_id,
                "candidate_digest": candidate,
                "status": "fail",
                "why_failed": why_failed,
                "where_failed": "observe.prove",
                "next_repair": "reconcile every law-bearing command",
                "claim_impact": "update_goal_blocked",
                "law_id": "full-local-observability-stack-integration-non-opaque-failure",
                "check_id": "full-local-observability-stack-integration-non-opaque-failure",
                "claim_id": "observability-control-plane",
                "query_hint_logql": "_time:5m operation:observe.prove",
                "query_hint_promql": "sum by (__name__,command,operation,status,law_id,check_id,claim_id,surface,failure_class,exporter,saturation_status) (last_over_time({__name__=~\"ultragoal_command_total|ultragoal_command_duration_ms|ultragoal_command_task_count|ultragoal_command_queue_depth|ultragoal_command_event_unix_seconds\",operation=\"observe.prove\"}[5m]))",
                "query_hint_traceql": "{operation=\"observe.prove\"}"
            })
        ),
    )
    .expect("spool event");
}

pub(super) fn write_command_receipt_event(root: &std::path::Path, run_id: &str) {
    let dir = root.join("validation_artifacts/observability");
    fs::create_dir_all(&dir).expect("observability dir");
    let candidate = crate::package::inventory::package_digest(root).expect("digest");
    crate::json_boundary::write_json(
        &dir.join("mandatory-law-validation.json"),
        &json!({
            "run_id": run_id,
            "candidate_digest": candidate,
            "status": "fail",
            "event": {
                "run_id": run_id,
                "candidate_digest": candidate,
                "status": "fail",
                "failure_class": "mandatory_law_validation_failure",
                "why_failed": "mandatory law validation failed",
                "where_failed": "mandatory-law.validation",
                "next_repair": "query logs metrics traces then repair the named law",
                "claim_impact": "readiness_blocked",
                "law_id": "full-local-observability-stack-integration-non-opaque-failure",
                "check_id": "mandatory-law-validation-observability-binding",
                "claim_id": "mandatory_law_validation"
            }
        }),
    )
    .expect("command receipt event");
}

pub(super) fn write_query_receipt_event(root: &std::path::Path, run_id: &str) {
    let dir = root.join("validation_artifacts/observability");
    fs::create_dir_all(&dir).expect("observability dir");
    let candidate = crate::package::inventory::package_digest(root).expect("digest");
    crate::json_boundary::write_json(
        &dir.join("zz-traces-query.json"),
        &json!({
            "schema": crate::cli::observe::types::QUERY_SCHEMA,
            "run_id": run_id,
            "candidate_digest": candidate,
            "status": "pass",
            "event": {
                "run_id": run_id,
                "candidate_digest": candidate,
                "operation": "observe.traces.query",
                "status": "pass",
                "failure_class": "none",
                "why_failed": "none",
                "where_failed": "none",
                "next_repair": "none",
                "claim_impact": "query_observation_only"
            }
        }),
    )
    .expect("query receipt event");
}

pub(super) fn write_failed_metrics_query_receipt(root: &std::path::Path, run_id: &str) {
    let dir = root.join("validation_artifacts/observability");
    fs::create_dir_all(&dir).expect("observability dir");
    let candidate = crate::package::inventory::package_digest(root).expect("digest");
    crate::json_boundary::write_json(
        &dir.join("metrics-query-failed.json"),
        &json!({
            "schema": crate::cli::observe::types::QUERY_SCHEMA,
            "query_kind": "metrics",
            "run_id": run_id,
            "candidate_digest": candidate,
            "status": "fail",
            "row_count": 1,
            "why_failed": "observability_metric_run_reconciliation_mismatch:duration_ms metric=930 target=128754",
            "where_failed": "observe.metrics.query",
            "metric_failure_class": "mandatory_law_validation_failure",
            "metric_error_count": 1,
            "query": "sum by (...)"
        }),
    )
    .expect("failed metrics query receipt");
}
