use crate::cli::observe::query::QueryKind;
use std::path::Path;

#[test]
fn retry_reports_last_transport_error_when_no_query_body_arrives() {
    let mut command = super::command();
    command.timeout_ms = 1;

    let error = super::super::retry_until_reconciled(
        &command,
        || Err("transport unavailable".to_string()),
        |_| Some("observability query returned no matching rows".to_string()),
    )
    .expect_err("transport error is retained");

    assert_eq!(error, "transport unavailable");
}

#[test]
fn retry_accepts_transient_trace_transport_error_before_backend_ingestion_catches_up() {
    let mut command = super::command();
    command.timeout_ms = 500;
    let mut attempts = 0;

    let body = super::super::retry_until_reconciled(
        &command,
        || {
            attempts += 1;
            if attempts == 1 {
                Err("observability_trace_backend_not_ready:trace-target".to_string())
            } else {
                Ok("{\"data\":[{\"traceID\":\"trace-target\"}]}".to_string())
            }
        },
        |_| None,
    )
    .expect("transient trace transport errors should retry within the bounded query window");

    assert_eq!(body, "{\"data\":[{\"traceID\":\"trace-target\"}]}");
    assert_eq!(attempts, 2);
}

#[test]
fn query_result_records_transport_errors_as_fail_closed_receipts() {
    let root = super::prepare_root("query-transport-error");

    let receipt = super::super::result_from_output(
        Path::new(&root),
        &super::command(),
        QueryKind::Logs,
        "run_id:run-query-bound".to_string(),
        Err("transport unavailable".to_string()),
    )
    .expect("transport failure receipt");

    assert_eq!(receipt["status"], "fail");
    assert_eq!(receipt["why_failed"], "transport unavailable");
    assert_eq!(receipt["row_count"], 0);
    std::fs::remove_dir_all(root).expect("cleanup transport error");
}

#[test]
fn retry_returns_specific_body_failures_but_not_empty_poll_results() {
    let mut command = super::command();
    command.timeout_ms = 1;
    let body = "{\"data\":{\"result\":[{}]}}".to_string();
    let result = super::super::retry_until_reconciled(
        &command,
        || Ok(body.clone()),
        |_| Some("observability_metric_failure_mismatch:none!=coverage".to_string()),
    )
    .expect("specific body failure is returned for receipt reconciliation");
    assert_eq!(result, body);

    let empty = super::super::retry_until_reconciled(
        &command,
        || Ok("{\"data\":[]}".to_string()),
        |_| Some("observability query returned no matching rows".to_string()),
    )
    .expect_err("empty polling results are not accepted");
    assert_eq!(empty, "observability query returned no matching rows");
}
