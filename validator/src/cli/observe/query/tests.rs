use crate::cli::observe::types::{ObserveCommand, ObserveOperation};
use serde_json::json;
use std::path::Path;

fn prepare_root(label: &str) -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::support::temp_root(label);
    std::fs::create_dir_all(&root).expect("query temp root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("query manifest");
    root
}

fn command() -> ObserveCommand {
    ObserveCommand {
        operation: ObserveOperation::LogsQuery,
        receipt: None,
        query: None,
        run_id: Some("run-query-bound".to_string()),
        correlation_id: None,
        claim_id: None,
        check_id: None,
        law_id: None,
        row_limit: 100,
        byte_limit: 1024,
        timeout_ms: 1000,
    }
}

fn metrics_command() -> ObserveCommand {
    let mut command = command();
    command.operation = ObserveOperation::MetricsQuery;
    command
}

#[test]
fn rejects_zero_row_limit_as_unbounded_query() {
    let root = prepare_root("query-row-limit-zero");
    let mut command = command();
    command.row_limit = 0;

    let receipt = super::run(Path::new(&root), &command).expect("query receipt");

    assert_eq!(receipt["status"], "fail");
    assert_eq!(
        receipt["why_failed"],
        "unbounded observability query rejected"
    );
    std::fs::remove_dir_all(root).expect("cleanup query row limit");
}

#[test]
fn rejects_zero_byte_limit_and_timeout_as_unbounded_queries() {
    for (name, bound) in [
        ("query-byte-limit-zero", "byte"),
        ("query-timeout-zero", "timeout"),
    ] {
        let root = prepare_root(name);
        let mut command = command();
        if bound == "byte" {
            command.byte_limit = 0;
        } else {
            command.timeout_ms = 0;
        }

        let receipt = super::run(Path::new(&root), &command).expect("query receipt");

        assert_eq!(receipt["status"], "fail");
        assert_eq!(
            receipt["why_failed"],
            "unbounded observability query rejected"
        );
        std::fs::remove_dir_all(root).expect("cleanup query bound");
    }
}

#[test]
fn metrics_query_result_does_not_require_candidate_digest_in_rows() {
    let root = prepare_root("query-metrics-no-candidate-row");
    let mut command = metrics_command();
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

    let receipt = super::result_from_output(
        Path::new(&root),
        &command,
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
    let root = prepare_root("query-metrics-operation-mismatch");
    write_target_event(&root, "observe.prove", "observability_gate_failure");
    let mut command = metrics_command();
    command.correlation_id = Some("corr-query-bound".to_string());
    let body = metric_body("archive.build", "archive_build_failure");

    let receipt = super::result_from_output(
        Path::new(&root),
        &command,
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
    let root = prepare_root("query-metrics-operation-match");
    write_target_event(&root, "observe.prove", "observability_gate_failure");
    let mut command = metrics_command();
    command.correlation_id = Some("corr-query-bound".to_string());
    let body = metric_body("observe.prove", "observability_gate_failure");

    let receipt = super::result_from_output(
        Path::new(&root),
        &command,
        "sum by (...)".to_string(),
        Ok(body),
    )
    .expect("metrics match receipt");

    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["metric_operation"], "observe.prove");
    assert_eq!(
        receipt["metric_failure_class"],
        "observability_gate_failure"
    );
    std::fs::remove_dir_all(root).expect("cleanup metrics match");
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

#[test]
fn query_result_reports_package_digest_errors_before_claiming_rows() {
    let root = crate::self_tests::boundaries::support::temp_root("query-missing-manifest");
    std::fs::create_dir_all(&root).expect("query missing manifest root");
    let err = super::result_from_output(
        Path::new(&root),
        &command(),
        "run_id:run-query-bound".to_string(),
        Ok("{\"data\":[]}".to_string()),
    )
    .expect_err("missing manifest blocks query receipt");

    assert!(err.contains("plugin-manifest-draft.json"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup missing manifest query");
}
