use crate::cli::observe::types::{ObserveCommand, ObserveOperation};
use serde_json::json;
use std::path::Path;

fn command() -> ObserveCommand {
    ObserveCommand {
        operation: ObserveOperation::MetricsQuery,
        receipt: None,
        query: None,
        run_id: Some("run-metrics-edge".to_string()),
        correlation_id: None,
        claim_id: None,
        check_id: None,
        law_id: None,
        row_limit: 100,
        byte_limit: 4096,
        timeout_ms: 100,
    }
}

#[test]
fn target_query_uses_target_event_status_and_respects_explicit_query() {
    let root = super::super::prepare_root("observe-metrics-target-query");
    write_target_event(
        &root,
        "coverage.prove",
        "fail",
        "coverage_prove_failure",
        None,
    );
    let mut target = command();
    let query = super::super::super::metrics::target_query(Path::new(&root), &target)
        .expect("target failure query");
    assert!(query.contains("coverage.prove"));
    assert!(query.contains("failure_class"));

    target.query = Some("custom".to_string());
    assert_eq!(
        super::super::super::metrics::target_query(Path::new(&root), &target),
        None
    );
    target.query = None;
    target.operation = ObserveOperation::LogsQuery;
    assert_eq!(
        super::super::super::metrics::target_query(Path::new(&root), &target),
        None
    );

    let root_pass = super::super::prepare_root("observe-metrics-target-query-pass");
    write_target_event(&root_pass, "package.digest", "pass", "none", None);
    let query = super::super::super::metrics::target_query(Path::new(&root_pass), &command())
        .expect("target pass query");
    assert!(query.contains("package.digest"));
    assert!(!query.contains("failure_class=\""));
    std::fs::remove_dir_all(root).expect("cleanup target query");
    std::fs::remove_dir_all(root_pass).expect("cleanup target query pass");
}

#[test]
fn target_query_without_target_event_stays_unfitted_instead_of_guessing_operation() {
    let root = super::super::prepare_root("observe-metrics-target-query-missing");

    assert_eq!(
        super::super::super::metrics::target_query(Path::new(&root), &command()),
        None
    );
    std::fs::remove_dir_all(root).expect("cleanup missing target query");
}

#[test]
fn metrics_reconciliation_fails_for_missing_target_and_wrong_candidate() {
    let root = super::super::prepare_root("observe-metrics-target-missing");
    let receipt = super::super::super::result_from_output(
        Path::new(&root),
        &command(),
        "sum by (...)".to_string(),
        Ok(metric_body("coverage.prove", "coverage_prove_failure")),
    )
    .expect("target missing receipt");
    assert_eq!(receipt["status"], "fail");
    assert!(
        receipt["why_failed"]
            .as_str()
            .unwrap()
            .starts_with("observability_metric_target_unavailable:run_id=run-metrics-edge")
    );

    let root = super::super::prepare_root("observe-metrics-candidate-mismatch");
    write_target_event(
        &root,
        "coverage.prove",
        "fail",
        "coverage_prove_failure",
        Some("sha256:old"),
    );
    let receipt = super::super::super::result_from_output(
        Path::new(&root),
        &command(),
        "sum by (...)".to_string(),
        Ok(metric_body("coverage.prove", "coverage_prove_failure")),
    )
    .expect("candidate mismatch receipt");
    assert_eq!(receipt["status"], "fail");
    assert!(
        receipt["why_failed"]
            .as_str()
            .unwrap()
            .starts_with("observability_metric_candidate_mismatch:sha256:old!=")
    );
    std::fs::remove_dir_all(root).expect("cleanup candidate mismatch");
}

#[test]
fn metrics_reconciliation_reports_missing_operation_and_error_count() {
    let root = super::super::prepare_root("observe-metrics-missing-operation");
    write_target_event(&root, "", "fail", "coverage_prove_failure", None);
    let receipt = super::super::super::result_from_output(
        Path::new(&root),
        &command(),
        "sum by (...)".to_string(),
        Ok(metric_body("", "coverage_prove_failure")),
    )
    .expect("missing operation receipt");
    assert_eq!(
        receipt["why_failed"],
        "observability_metric_missing_for_target:unknown"
    );

    let root = super::super::prepare_root("observe-metrics-error-count");
    write_target_event(
        &root,
        "coverage.prove",
        "fail",
        "coverage_prove_failure",
        None,
    );
    let receipt = super::super::super::result_from_output(
        Path::new(&root),
        &command(),
        "sum by (...)".to_string(),
        Ok(metric_body_without_total(
            "coverage.prove",
            "coverage_prove_failure",
        )),
    )
    .expect("missing error count receipt");
    assert_eq!(
        receipt["why_failed"],
        "observability_metric_error_count_missing:coverage_prove_failure"
    );
    std::fs::remove_dir_all(root).expect("cleanup missing error count");
}

#[test]
fn metrics_reconciliation_ignores_unreported_optional_runtime_signals() {
    let root = super::super::prepare_root("observe-metrics-optional-runtime");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    crate::cli::observe::telemetry::spool_write_for_test(
        &root,
        &json!({
            "schema": crate::cli::observe::types::EVENT_SCHEMA,
            "run_id": "run-metrics-edge",
            "candidate_digest": candidate,
            "operation": "coverage.prove",
            "status": "pass",
            "failure_class": "none"
        }),
    )
    .expect("target event");
    let receipt = super::super::super::result_from_output(
        Path::new(&root),
        &command(),
        "sum by (...)".to_string(),
        Ok(pass_metric_body("coverage.prove")),
    )
    .expect("optional runtime receipt");

    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["metric_operation"], "coverage.prove");
    std::fs::remove_dir_all(root).expect("cleanup optional runtime");
}

fn write_target_event(
    root: &Path,
    operation: &str,
    status: &str,
    failure_class: &str,
    candidate: Option<&str>,
) {
    let candidate = candidate
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| crate::package::inventory::package_digest(root).expect("candidate"));
    let event = json!({
        "schema": crate::cli::observe::types::EVENT_SCHEMA,
        "run_id": "run-metrics-edge",
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

fn metric_body(operation: &str, failure_class: &str) -> String {
    metric_body_with_status(operation, "fail", failure_class)
}

fn pass_metric_body(operation: &str) -> String {
    metric_body_with_status(operation, "pass", "none")
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

fn metric_body_without_total(operation: &str, failure_class: &str) -> String {
    json!({
        "status": "success",
        "data": {"result": [{
            "metric": {
                "__name__": "ultragoal_command_duration_ms",
                "operation": operation,
                "status": "fail",
                "failure_class": failure_class,
                "saturation_status": "serial_command_typed"
            },
            "value": [1, "10"]
        }]}
    })
    .to_string()
}
