use crate::cli::observe::query::QueryKind;
use crate::cli::observe::types::{ObserveCommand, ObserveOperation};
use serde_json::json;
use std::path::Path;

fn command() -> ObserveCommand {
    ObserveCommand {
        operation: ObserveOperation::MetricsQuery,
        receipt: None,
        query: None,
        run_id: Some("run-metrics-freshness".to_string()),
        correlation_id: None,
        claim_id: None,
        check_id: None,
        law_id: None,
        target_command: None,
        target_family: None,
        row_limit: 100,
        byte_limit: 4096,
        timeout_ms: 100,
    }
}

#[test]
fn metrics_query_retry_waits_for_reconciled_rows() {
    let command = super::super::metrics_command();
    let mut attempts = 0;

    let body = super::super::super::retry_until_reconciled(
        &command,
        || {
            attempts += 1;
            Ok(format!("body-{attempts}"))
        },
        |body| {
            (body == "body-1").then(|| {
                "observability_metric_run_reconciliation_mismatch:duration_ms metric=1 target=2"
                    .to_string()
            })
        },
    )
    .expect("retry reaches reconciled body");

    assert_eq!(body, "body-2");
    assert_eq!(attempts, 2);
}

#[test]
fn accepts_same_window_pass_metrics() {
    let root = super::super::prepare_root("observe-metrics-fresh-pass");
    write_target_event_with_timestamp(&root, "coverage.prove", "2026-07-01T00:00:00Z");
    let receipt = super::super::super::result_from_output(
        Path::new(&root),
        &command(),
        QueryKind::Metrics,
        "sum by (...)".to_string(),
        Ok(metric_body_with_timestamp("coverage.prove", 1_782_864_000)),
    )
    .expect("fresh receipt");

    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["metric_operation"], "coverage.prove");
    std::fs::remove_dir_all(root).expect("cleanup fresh pass");
}

#[test]
fn rejects_stale_operation_metric_even_when_duration_is_high_enough() {
    let root = super::super::prepare_root("observe-metrics-stale-event-time");
    write_target_event_with_timestamp(&root, "coverage.prove", "2026-07-01T00:01:00Z");
    let receipt = super::super::super::result_from_output(
        Path::new(&root),
        &command(),
        QueryKind::Metrics,
        "sum by (...)".to_string(),
        Ok(metric_body_with_event_time(
            "coverage.prove",
            1_782_864_000,
            1_782_863_940,
            99,
        )),
    )
    .expect("stale metric event time receipt");

    assert_eq!(receipt["status"], "fail");
    assert_eq!(
        receipt["why_failed"],
        "observability_metric_event_time_stale:metric_event_unix=1782863940 target_event_unix=1782864060"
    );
    std::fs::remove_dir_all(root).expect("cleanup stale metric event time");
}

#[test]
fn metrics_query_reports_target_run_outside_bounded_window() {
    let root = super::super::prepare_root("query-metrics-stale-target-window");
    write_target_event(&root);
    let mut command = super::super::metrics_command();
    command.correlation_id = Some("corr-query-bound".to_string());
    command.byte_limit = 4096;

    let receipt = crate::cli::observe::query::result_from_output(
        Path::new(&root),
        &command,
        QueryKind::Metrics,
        "sum by (...)".to_string(),
        Ok(metric_body()),
    )
    .expect("metrics stale target receipt");

    assert_eq!(receipt["status"], "fail");
    assert_eq!(
        receipt["why_failed"],
        "observability_metric_target_outside_query_window:target_timestamp=2026-07-01T19:42:39Z latest_metric_timestamp=2026-07-01T19:48:00Z window_seconds=300"
    );
    assert_eq!(receipt["metric_latest_sample_unix"], 1782935280);
    std::fs::remove_dir_all(root).expect("cleanup stale target metrics");
}

fn write_target_event_with_timestamp(root: &Path, operation: &str, timestamp: &str) {
    let candidate = crate::package::inventory::package_digest(root).expect("candidate");
    let event = json!({
        "schema": crate::cli::observe::types::EVENT_SCHEMA,
        "run_id": "run-metrics-freshness",
        "candidate_digest": candidate,
        "operation": operation,
        "status": "pass",
        "failure_class": "none",
        "duration_ms": 1,
        "task_count": 1,
        "queue_depth": 1,
        "timestamp": timestamp
    });
    crate::cli::observe::telemetry::spool_write_for_test(root, &event).expect("spool event");
}

fn write_target_event(root: &Path) {
    let candidate = crate::package::inventory::package_digest(root).expect("candidate");
    let event = json!({
        "schema": crate::cli::observe::types::EVENT_SCHEMA,
        "run_id": "run-query-bound",
        "correlation_id": "corr-query-bound",
        "candidate_digest": candidate,
        "operation": "source.audit",
        "status": "fail",
        "failure_class": "source_audit_check_failure",
        "why_failed": "source audit failed for a specific reason",
        "where_failed": "source.audit",
        "next_repair": "rerun source audit for current metrics proof",
        "claim_impact": "readiness_release_completion_update_goal_blocked",
        "law_id": crate::cli::observe::types::LAW_ID,
        "check_id": crate::cli::observe::types::CHECK_ID,
        "claim_id": crate::cli::observe::types::CLAIM_ID,
        "timestamp": "2026-07-01T19:42:39Z",
        "duration_ms": 130043,
        "task_count": 2695,
        "queue_depth": 2695
    });
    crate::cli::observe::telemetry::spool_write_for_test(root, &event).expect("spool event");
}

fn metric_body_with_timestamp(operation: &str, timestamp: i64) -> String {
    metric_body_with_event_time(operation, timestamp, timestamp, 1)
}

fn metric_body_with_event_time(
    operation: &str,
    sample_timestamp: i64,
    event_timestamp: i64,
    duration_ms: u64,
) -> String {
    let labels = json!({
        "operation": operation,
        "status": "pass",
        "failure_class": "none",
        "saturation_status": "serial_command_typed"
    });
    json!({
        "status": "success",
        "data": {"result": [
            {"metric": metric_labels(&labels, "ultragoal_command_total"), "value": [sample_timestamp, "1"]},
            {"metric": metric_labels(&labels, "ultragoal_command_duration_ms"), "value": [sample_timestamp, duration_ms.to_string()]},
            {"metric": metric_labels(&labels, "ultragoal_command_task_count"), "value": [sample_timestamp, "1"]},
            {"metric": metric_labels(&labels, "ultragoal_command_queue_depth"), "value": [sample_timestamp, "1"]},
            {"metric": metric_labels(&labels, "ultragoal_command_event_unix_seconds"), "value": [sample_timestamp, event_timestamp.to_string()]}
        ]}
    })
    .to_string()
}

fn metric_body() -> String {
    let labels = json!({
        "operation": "source.audit",
        "status": "fail",
        "failure_class": "source_audit_check_failure",
        "saturation_status": "queued_parallel_work"
    });
    json!({
        "status": "success",
        "data": {
            "result": [
                {"metric": metric_labels(&labels, "ultragoal_command_total"), "value": [1782935280, "1"]},
                {"metric": metric_labels(&labels, "ultragoal_command_duration_ms"), "value": [1782935280, "893"]},
                {"metric": metric_labels(&labels, "ultragoal_command_task_count"), "value": [1782935280, "55"]},
                {"metric": metric_labels(&labels, "ultragoal_command_queue_depth"), "value": [1782935280, "55"]},
                {"metric": metric_labels(&labels, "ultragoal_command_event_unix_seconds"), "value": [1782935280, "1782934959"]}
            ]
        }
    })
    .to_string()
}

fn metric_labels(labels: &serde_json::Value, metric_name: &str) -> serde_json::Value {
    let mut out = labels.as_object().cloned().unwrap_or_default();
    out.insert("__name__".to_string(), json!(metric_name));
    serde_json::Value::Object(out)
}
