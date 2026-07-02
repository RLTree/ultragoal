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
fn query_result_records_transport_errors_as_fail_closed_receipts() {
    let root = super::prepare_root("query-transport-error");

    let receipt = super::super::result_from_output(
        Path::new(&root),
        &super::command(),
        "run_id:run-query-bound".to_string(),
        Err("transport unavailable".to_string()),
    )
    .expect("transport failure receipt");

    assert_eq!(receipt["status"], "fail");
    assert_eq!(receipt["why_failed"], "transport unavailable");
    assert_eq!(receipt["row_count"], 0);
    std::fs::remove_dir_all(root).expect("cleanup transport error");
}
