use crate::cli::observe::query::QueryKind;
use serde_json::json;
use std::path::Path;

mod edges;
mod freshness;
mod reconciliation;

const OBSERVABILITY_CLOSURE_FAILURE: &str = "observability_product_closure_failure";

#[test]
fn metrics_query_result_does_not_require_candidate_digest_in_rows() {
    let root = super::prepare_root("query-metrics-no-candidate-row");
    let mut command = super::metrics_command();
    command.run_id = None;
    let body = json!({
        "status": "success",
        "data": {
            "result": [{
                "metric": {
                    "__name__": "ultragoal_command_total",
                    "operation": "coverage.prove"
                },
                "value": [1, "1"]
            }]
        }
    })
    .to_string();

    let receipt = crate::cli::observe::query::result_from_output(
        Path::new(&root),
        &command,
        QueryKind::Metrics,
        "max_over_time(ultragoal_command_total[24h])".to_string(),
        Ok(body),
    )
    .expect("metrics query receipt");

    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["query_kind"], "metrics");
    std::fs::remove_dir_all(root).expect("cleanup metrics query");
}

#[test]
fn metrics_query_rejects_unrelated_operation_for_requested_run() {
    let root = super::prepare_root("query-metrics-operation-mismatch");
    write_target_event(&root, "observe.prove", OBSERVABILITY_CLOSURE_FAILURE);
    let mut command = super::metrics_command();
    command.correlation_id = Some("corr-query-bound".to_string());
    let body = metric_body("archive.build", "archive_build_failure");

    let receipt = crate::cli::observe::query::result_from_output(
        Path::new(&root),
        &command,
        QueryKind::Metrics,
        "sum by (...)".to_string(),
        Ok(body),
    )
    .expect("metrics mismatch receipt");

    assert_eq!(receipt["status"], "fail");
    assert_eq!(
        receipt["why_failed"],
        "observability_metric_operation_mismatch:archive.build!=observe.prove"
    );
    assert_eq!(receipt["metric_operation"], "archive.build");
    std::fs::remove_dir_all(root).expect("cleanup metrics mismatch");
}

#[test]
fn metrics_query_accepts_target_operation_failure_signal() {
    let root = super::prepare_root("query-metrics-operation-match");
    write_target_event(&root, "observe.prove", OBSERVABILITY_CLOSURE_FAILURE);
    let mut command = super::metrics_command();
    command.correlation_id = Some("corr-query-bound".to_string());
    let body = metric_body("observe.prove", OBSERVABILITY_CLOSURE_FAILURE);

    let receipt = crate::cli::observe::query::result_from_output(
        Path::new(&root),
        &command,
        QueryKind::Metrics,
        "sum by (...)".to_string(),
        Ok(body),
    )
    .expect("metrics match receipt");

    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["metric_operation"], "observe.prove");
    assert_eq!(
        receipt["metric_failure_class"],
        OBSERVABILITY_CLOSURE_FAILURE
    );
    std::fs::remove_dir_all(root).expect("cleanup metrics match");
}

#[test]
fn metrics_query_rejects_underreported_target_run_signals() {
    let root = super::prepare_root("query-metrics-underreported-run");
    write_target_event_with_runtime(
        &root,
        "source.audit",
        "source_audit_check_failure",
        RuntimeSignals {
            duration_ms: 2000,
            task_count: 55,
            queue_depth: 55,
            saturation_status: "queued_parallel_work",
        },
    );
    let mut command = super::metrics_command();
    command.correlation_id = Some("corr-query-bound".to_string());
    let body = metric_body_with_signals(
        "source.audit",
        "source_audit_check_failure",
        RuntimeSignals {
            duration_ms: 874,
            task_count: 10,
            queue_depth: 10,
            saturation_status: "queued_parallel_work",
        },
    );

    let receipt = crate::cli::observe::query::result_from_output(
        Path::new(&root),
        &command,
        QueryKind::Metrics,
        "sum by (...)".to_string(),
        Ok(body),
    )
    .expect("metrics underreport receipt");

    assert_eq!(receipt["status"], "fail");
    assert_eq!(
        receipt["why_failed"],
        "observability_metric_run_reconciliation_mismatch:duration_ms metric=874 target=2000"
    );
    assert_eq!(receipt["metric_latency_ms"], 874);
    assert_eq!(receipt["metric_task_count"], 10);
    assert_eq!(receipt["metric_queue_depth"], 10);
    std::fs::remove_dir_all(root).expect("cleanup metrics underreport");
}

fn write_target_event(root: &Path, operation: &str, failure_class: &str) {
    let candidate = crate::package::inventory::package_digest(root).expect("candidate");
    let event = json!({
        "schema": crate::cli::observe::types::EVENT_SCHEMA,
        "run_id": "run-query-bound",
        "correlation_id": "corr-query-bound",
        "candidate_digest": candidate,
        "operation": operation,
        "status": "fail",
        "failure_class": failure_class,
        "why_failed": "target command failed for a specific reason",
        "where_failed": operation,
        "next_repair": "repair the target operation and rerun narrowly",
        "claim_impact": "readiness_release_completion_update_goal_blocked",
        "law_id": crate::cli::observe::types::LAW_ID,
        "check_id": crate::cli::observe::types::CHECK_ID,
        "claim_id": crate::cli::observe::types::CLAIM_ID
    });
    crate::cli::observe::telemetry::spool_write_for_test(root, &event).expect("spool event");
}

struct RuntimeSignals {
    duration_ms: u64,
    task_count: u64,
    queue_depth: u64,
    saturation_status: &'static str,
}

fn write_target_event_with_runtime(
    root: &Path,
    operation: &str,
    failure_class: &str,
    runtime: RuntimeSignals,
) {
    let candidate = crate::package::inventory::package_digest(root).expect("candidate");
    let event = json!({
        "schema": crate::cli::observe::types::EVENT_SCHEMA,
        "run_id": "run-query-bound",
        "correlation_id": "corr-query-bound",
        "candidate_digest": candidate,
        "operation": operation,
        "status": "fail",
        "failure_class": failure_class,
        "why_failed": "target command failed for a specific reason",
        "where_failed": operation,
        "next_repair": "repair the target operation and rerun narrowly",
        "claim_impact": "readiness_release_completion_update_goal_blocked",
        "law_id": crate::cli::observe::types::LAW_ID,
        "check_id": crate::cli::observe::types::CHECK_ID,
        "claim_id": crate::cli::observe::types::CLAIM_ID,
        "duration_ms": runtime.duration_ms,
        "task_count": runtime.task_count,
        "queue_depth": runtime.queue_depth,
        "saturation_status": runtime.saturation_status
    });
    crate::cli::observe::telemetry::spool_write_for_test(root, &event).expect("spool event");
}

fn metric_body(operation: &str, failure_class: &str) -> String {
    json!({
        "status": "success",
        "data": {
            "result": [{
                "metric": {
                    "__name__": "ultragoal_command_total",
                    "operation": operation,
                    "status": "fail",
                    "failure_class": failure_class,
                    "saturation_status": "serial_command_typed"
                },
                "value": [1, "1"]
            }]
        }
    })
    .to_string()
}

fn metric_body_with_signals(
    operation: &str,
    failure_class: &str,
    runtime: RuntimeSignals,
) -> String {
    let labels = json!({
        "operation": operation,
        "status": "fail",
        "failure_class": failure_class,
        "saturation_status": runtime.saturation_status
    });
    json!({
        "status": "success",
        "data": {
            "result": [
                {"metric": metric_labels(&labels, "ultragoal_command_total"), "value": [1, "1"]},
                {"metric": metric_labels(&labels, "ultragoal_command_duration_ms"), "value": [1, runtime.duration_ms.to_string()]},
                {"metric": metric_labels(&labels, "ultragoal_command_task_count"), "value": [1, runtime.task_count.to_string()]},
                {"metric": metric_labels(&labels, "ultragoal_command_queue_depth"), "value": [1, runtime.queue_depth.to_string()]}
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
