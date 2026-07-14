use super::super::*;

#[test]
fn trace_backend_request_uses_direct_lookup_when_target_event_has_trace_id() {
    let root = prepare_root("trace-transport-target-id");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    crate::cli::observe::telemetry::spool_write_for_test(
        &root,
        &json!({
            "schema": crate::cli::observe::command::EVENT_SCHEMA,
            "run_id": "run-query-bound",
            "correlation_id": "corr-query-bound",
            "candidate_digest": candidate,
            "trace_id": "trace-target-id",
            "operation": "loop.measure.fmt_check",
            "status": "fail",
            "failure_class": "observability_trace_tree_unavailable"
        }),
    )
    .expect("target event");

    let request = super::super::super::transport::trace_backend_request_for_test(
        &root,
        &command(),
        "{operation=\"loop.measure.fmt_check\"}",
    );

    assert!(request.starts_with("trace_by_id:"), "{request}");
    std::fs::remove_dir_all(root).expect("cleanup trace target id");
}

#[test]
fn trace_transport_bounded_backend_errors_are_agent_legible() {
    let root = prepare_root("trace-transport-bounded-errors");
    let mut command = command();
    command.operation = crate::cli::observe::command::ObserveOperation::TracesQuery;
    command.timeout_ms = 1;

    let search_error =
        super::super::super::transport::curl_traces(&root, &command, "{run_id=\"missing\"}")
            .expect_err("trace backend is unavailable in unit test");

    assert!(
        search_error.contains("curl query failed")
            || search_error.contains("victoriatraces")
            || search_error.contains("curl launch failed"),
        "{search_error}"
    );
    assert_eq!(
        super::super::super::transport::trace_attempt_timeout_seconds_for_test(5_000),
        "1"
    );
    std::fs::remove_dir_all(root).expect("cleanup trace backend errors");
}

#[test]
fn traces_query_uses_target_run_and_correlation_tags_when_target_event_exists() {
    let root = prepare_root("query-target-trace-id");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    write_trace_target_event(&root, &candidate);
    let mut command = command();
    command.operation = crate::cli::observe::command::ObserveOperation::TracesQuery;

    let query = super::super::super::transport::trace_query_for_test(&root, &command);
    let operation = super::super::super::transport::trace_operation_for_test(&root, &command);

    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&query).expect("trace tag json"),
        serde_json::json!({"correlation_id":"corr-query-bound","run_id":"run-query-bound"})
    );
    assert_eq!(operation.as_deref(), Some("source.audit"));
    std::fs::remove_dir_all(root).expect("cleanup trace query target");
}

#[test]
fn trace_transport_caps_each_attempt_inside_larger_retry_window() {
    assert_eq!(
        super::super::super::transport::trace_attempt_timeout_seconds_for_test(3_000),
        "1"
    );
    assert_eq!(
        super::super::super::transport::trace_attempt_timeout_seconds_for_test(250),
        "0.25"
    );
}

#[test]
fn target_trace_query_uses_exact_backend_trace_id_not_broad_search() {
    let root = prepare_root("query-target-trace-backend-id");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    write_trace_target_event(&root, &candidate);
    let mut command = command();
    command.operation = crate::cli::observe::command::ObserveOperation::TracesQuery;

    let request =
        super::super::super::transport::trace_backend_request_for_test(&root, &command, "{}");
    let backend_id = super::super::super::transport::trace_backend_id_for_test("trace-query-bound");

    assert_eq!(request, format!("trace_by_id:{backend_id}"));
    std::fs::remove_dir_all(root).expect("cleanup trace backend id target");
}

#[test]
fn trace_backend_request_uses_bounded_search_when_no_target_event_exists() {
    let root = prepare_root("trace-transport-no-target-search");
    let mut command = command();
    command.operation = crate::cli::observe::command::ObserveOperation::TracesQuery;

    let request = super::super::super::transport::trace_backend_request_for_test(
        &root,
        &command,
        "{run_id=\"run-search\"}",
    );

    assert_eq!(request, "search::{run_id=\"run-search\"}");
    std::fs::remove_dir_all(root).expect("cleanup no target search");
}

#[test]
fn trace_query_falls_back_to_command_tags_when_target_event_lacks_run_and_correlation() {
    let root = prepare_root("trace-transport-target-without-tags");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    crate::cli::observe::telemetry::spool_write_for_test(
        &root,
        &json!({
            "schema": crate::cli::observe::command::EVENT_SCHEMA,
            "run_id": "none",
            "correlation_id": "",
            "candidate_digest": candidate,
            "trace_id": "trace-without-tags",
            "operation": "source.audit",
            "status": "pass",
            "failure_class": "none"
        }),
    )
    .expect("target event without query tags");
    let mut command = command();
    command.operation = crate::cli::observe::command::ObserveOperation::TracesQuery;

    let query = super::super::super::transport::trace_query_for_test(&root, &command);

    assert!(query.contains("run_id"));
    assert!(!query.contains("correlation_id"));
    assert_ne!(query, "{}");
    std::fs::remove_dir_all(root).expect("cleanup trace target without tags");
}

#[test]
fn trace_target_tags_are_absent_when_event_has_no_usable_run_or_correlation() {
    let tags = super::super::super::transport::trace_target_tags_for_test(&json!({
        "run_id": "none",
        "correlation_id": "",
        "trace_id": "trace-without-tags"
    }));

    assert_eq!(tags, None);
}

fn write_trace_target_event(root: &std::path::Path, candidate: &str) {
    crate::cli::observe::telemetry::spool_write_for_test(
        root,
        &json!({
            "schema": crate::cli::observe::command::EVENT_SCHEMA,
            "run_id": "run-query-bound",
            "correlation_id": "corr-query-bound",
            "candidate_digest": candidate,
            "trace_id": "trace-query-bound",
            "operation": "source.audit",
            "status": "pass",
            "failure_class": "none",
            "law_id": "law-query-bound",
            "check_id": "check-query-bound",
            "claim_id": "claim-query-bound"
        }),
    )
    .expect("target event");
}
