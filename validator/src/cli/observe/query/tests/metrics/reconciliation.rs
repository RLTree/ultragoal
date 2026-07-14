use crate::cli::observe::{
    command::{ObserveCommand, ObserveOperation},
    query::QueryKind,
};
use serde_json::json;
use std::path::Path;
fn command() -> ObserveCommand {
    ObserveCommand {
        operation: ObserveOperation::MetricsQuery,
        receipt: None,
        query: None,
        run_id: Some("run-metrics-reconcile".to_string()),
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
fn reports_candidate_missing_failure_mismatch_and_metric_absence() {
    let root = super::super::prepare_root("observe-metrics-candidate-missing");
    write_target_event_without_candidate(&root, "coverage.prove", "coverage_prove_failure");
    let receipt = super::super::super::result_from_output(
        Path::new(&root),
        &command(),
        QueryKind::Metrics,
        "sum by (...)".to_string(),
        Ok(metric_body("coverage.prove", "coverage_prove_failure")),
    )
    .expect("candidate missing receipt");
    assert_eq!(
        receipt["why_failed"],
        "observability_metric_candidate_missing"
    );
    let root = super::super::prepare_root("observe-metrics-failure-mismatch");
    write_target_event(&root, "coverage.prove", "fail", "coverage_prove_failure");
    let receipt = super::super::super::result_from_output(
        Path::new(&root),
        &command(),
        QueryKind::Metrics,
        "sum by (...)".to_string(),
        Ok(metric_body("coverage.prove", "source_audit_check_failure")),
    )
    .expect("failure mismatch receipt");
    assert_eq!(
        receipt["why_failed"],
        "observability_metric_failure_mismatch:source_audit_check_failure!=coverage_prove_failure"
    );
    let root = super::super::prepare_root("observe-metrics-operation-missing");
    write_target_event(&root, "coverage.prove", "pass", "none");
    let receipt = super::super::super::result_from_output(
        Path::new(&root),
        &command(),
        QueryKind::Metrics,
        "sum by (...)".to_string(),
        Ok(metric_body_without_operation()),
    )
    .expect("missing metric operation receipt");
    assert_eq!(
        receipt["why_failed"],
        "observability_metric_missing_for_target:coverage.prove"
    );
    std::fs::remove_dir_all(root).expect("cleanup operation missing");
}
#[test]
fn pass_target_uses_pass_only_query_and_rejects_error_metrics() {
    let root = super::super::prepare_root("observe-metrics-pass-target-error-signal");
    write_target_event(&root, "line-caps.check", "pass", "none");

    let query = super::super::super::metrics::target_query(Path::new(&root), &command())
        .expect("target query");
    assert!(query.contains("operation=\"line-caps.check\""), "{query}");
    assert!(query.contains("status=\"pass\""), "{query}");

    let receipt = super::super::super::result_from_output(
        Path::new(&root),
        &command(),
        QueryKind::Metrics,
        "sum by (...)".to_string(),
        Ok(metric_body("line-caps.check", "line_cap_failure")),
    )
    .expect("pass target rejects error metric");
    assert_eq!(receipt["status"], "fail");
    assert_eq!(
        receipt["why_failed"],
        "observability_metric_status_mismatch:fail!=pass"
    );
    assert_eq!(receipt["metric_status"], "fail");

    let receipt = super::super::super::result_from_output(
        Path::new(&root),
        &command(),
        QueryKind::Metrics,
        "sum by (...)".to_string(),
        Ok(metric_body_with_status(
            "line-caps.check",
            "pass",
            "line_cap_failure",
        )),
    )
    .expect("pass target rejects same-status error metric");
    assert_eq!(receipt["status"], "fail");
    assert_eq!(
        receipt["why_failed"],
        "observability_metric_pass_target_has_error_signal:line_cap_failure:error_count=1"
    );
    std::fs::remove_dir_all(root).expect("cleanup pass target error signal");
}
fn write_target_event(root: &Path, operation: &str, status: &str, failure_class: &str) {
    let candidate = crate::package::inventory::package_digest(root).expect("candidate");
    let event = json!({
        "schema": crate::cli::observe::command::EVENT_SCHEMA,
        "run_id": "run-metrics-reconcile",
        "candidate_digest": candidate,
        "operation": operation,
        "status": status,
        "failure_class": failure_class,
        "duration_ms": 10,
        "task_count": 1,
        "queue_depth": 1
    });
    crate::cli::observe::telemetry::spool_write_for_test(root, &event).expect("spool event");
}
fn write_target_event_without_candidate(root: &Path, operation: &str, failure_class: &str) {
    let event = json!({
        "schema": crate::cli::observe::command::EVENT_SCHEMA,
        "run_id": "run-metrics-reconcile",
        "operation": operation,
        "status": "fail",
        "failure_class": failure_class,
        "duration_ms": 10,
        "task_count": 1,
        "queue_depth": 1
    });
    crate::cli::observe::telemetry::spool_write_for_test(root, &event).expect("spool event");
}
fn metric_body(operation: &str, failure_class: &str) -> String {
    metric_body_with_status(operation, "fail", failure_class)
}
fn metric_body_with_status(operation: &str, status: &str, failure_class: &str) -> String {
    json!({
        "status": "success",
        "data": {"result": [{
            "metric": {
                "__name__": "ultragoal_command_total",
                "operation": operation,
                "status": status,
                "failure_class": failure_class,
                "saturation_status": "serial_command_typed"
            },
            "value": [1, "1"]
        }]}
    })
    .to_string()
}
fn metric_body_without_operation() -> String {
    json!({
        "status": "success",
        "data": {"result": [{
            "metric": {
                "__name__": "ultragoal_command_total",
                "status": "pass",
                "failure_class": "none",
                "saturation_status": "serial_command_typed"
            },
            "value": [1, "1"]
        }]}
    })
    .to_string()
}
