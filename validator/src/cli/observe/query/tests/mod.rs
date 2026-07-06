use super::QueryKind;
use crate::cli::observe::types::{ObserveCommand, ObserveOperation};
use serde_json::json;
use std::path::Path;

mod metrics;
mod record_projection;
mod records;
mod retry_edges;
mod target;
mod target_receipts;

fn prepare_root(label: &str) -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
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
        target_command: None,
        target_family: None,
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

fn metric_body_with_signals(operation: &str) -> String {
    let labels = json!({
        "operation": operation,
        "status": "pass",
        "failure_class": "none",
        "saturation_status": "serial_command_typed"
    });
    json!({
        "status": "success",
        "data": {"result": [
            {"metric": metric_labels(&labels, "ultragoal_command_total"), "value": [1, "1"]},
            {"metric": metric_labels(&labels, "ultragoal_command_duration_ms"), "value": [1, "1"]},
            {"metric": metric_labels(&labels, "ultragoal_command_task_count"), "value": [1, "1"]},
            {"metric": metric_labels(&labels, "ultragoal_command_queue_depth"), "value": [1, "1"]}
        ]}
    })
    .to_string()
}

fn metric_labels(labels: &serde_json::Value, metric_name: &str) -> serde_json::Value {
    let mut out = labels.as_object().cloned().unwrap_or_default();
    out.insert("__name__".to_string(), json!(metric_name));
    serde_json::Value::Object(out)
}

#[test]
fn rejects_zero_row_limit_as_unbounded_query() {
    let root = prepare_root("query-row-limit-zero");
    let mut command = command();
    command.row_limit = 0;

    let receipt = super::run(Path::new(&root), &command, QueryKind::Logs).expect("query receipt");

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

        let receipt =
            super::run(Path::new(&root), &command, QueryKind::Logs).expect("query receipt");

        assert_eq!(receipt["status"], "fail");
        assert_eq!(
            receipt["why_failed"],
            "unbounded observability query rejected"
        );
        std::fs::remove_dir_all(root).expect("cleanup query bound");
    }
}

#[test]
fn query_result_reports_package_digest_errors_before_claiming_rows() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("query-missing-manifest");
    std::fs::create_dir_all(&root).expect("query missing manifest root");
    let err = super::result_from_output(
        Path::new(&root),
        &command(),
        QueryKind::Logs,
        "run_id:run-query-bound".to_string(),
        Ok("{\"data\":[]}".to_string()),
    )
    .expect_err("missing manifest blocks query receipt");

    assert!(err.contains("plugin-manifest-draft.json"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup missing manifest query");
}

#[test]
fn live_result_failure_accepts_non_metric_rows_and_reports_empty_results() {
    let root = prepare_root("query-live-result");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    crate::cli::observe::telemetry::spool_write_for_test(
        &root,
        &json!({
            "schema": crate::cli::observe::types::EVENT_SCHEMA,
            "run_id": "run-query-bound",
            "candidate_digest": candidate,
            "operation": "source.audit",
            "status": "pass",
            "failure_class": "none"
        }),
    )
    .expect("target event");

    assert_eq!(
        super::live_result_failure(
            &root,
            &metrics_command(),
            QueryKind::Metrics,
            "{\"data\":{\"result\":[]}}",
            &candidate
        ),
        Some("observability query returned no matching rows".to_string())
    );
    assert_eq!(
        super::live_result_failure(
            &root,
            &command(),
            QueryKind::Logs,
            &json!({"candidate_digest": candidate, "operation": "source.audit"}).to_string(),
            &candidate
        ),
        Some("observability_logs_target_record_mismatch:run_id:none!=run-query-bound".to_string())
    );

    std::fs::remove_dir_all(root).expect("cleanup live result");
}

#[test]
fn live_result_validator_reconciles_metric_rows_to_current_target() {
    let root = prepare_root("query-live-result-metrics-reconcile");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    crate::cli::observe::telemetry::spool_write_for_test(
        &root,
        &json!({
            "schema": crate::cli::observe::types::EVENT_SCHEMA,
            "run_id": "run-query-bound",
            "candidate_digest": candidate,
            "operation": "source.audit",
            "status": "pass",
            "failure_class": "none",
            "duration_ms": 1,
            "task_count": 1,
            "queue_depth": 1
        }),
    )
    .expect("target event");
    let command = metrics_command();
    let mut validate =
        super::live_result_validator_for_test(&root, &command, QueryKind::Metrics, &candidate);

    assert_eq!(validate(&metric_body_with_signals("source.audit")), None);
    assert_eq!(
        super::live_result_failure(
            &root,
            &command,
            QueryKind::Metrics,
            &metric_body_with_signals("source.audit"),
            &candidate
        ),
        None
    );
    drop(validate);

    std::fs::remove_dir_all(root).expect("cleanup live metrics result");
}

#[test]
fn retry_returns_specific_body_failures_but_not_empty_poll_results() {
    let mut command = command();
    command.timeout_ms = 1;
    let body = "{\"data\":{\"result\":[{}]}}".to_string();
    let result = super::retry_until_reconciled(
        &command,
        || Ok(body.clone()),
        |_| Some("observability_metric_failure_mismatch:none!=coverage".to_string()),
    )
    .expect("specific body failure is returned for receipt reconciliation");
    assert_eq!(result, body);

    let empty = super::retry_until_reconciled(
        &command,
        || Ok("{\"data\":[]}".to_string()),
        |_| Some("observability query returned no matching rows".to_string()),
    )
    .expect_err("empty polling results are not accepted");
    assert_eq!(empty, "observability query returned no matching rows");
}
