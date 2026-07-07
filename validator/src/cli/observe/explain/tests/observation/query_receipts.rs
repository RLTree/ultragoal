use serde_json::json;
use std::fs;

#[test]
fn explain_targets_failed_durable_query_receipt_when_spool_is_absent() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "observe-explain-durable-query-failure",
    );
    fs::create_dir_all(root.join("validation_artifacts/observability/live-loop/commands"))
        .expect("receipt dir");
    fs::write(root.join("owned.txt"), "owned").expect("owned");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["owned.txt"]}),
    )
    .expect("manifest");
    let candidate = crate::package::inventory::package_digest(&root).expect("digest");
    crate::json_boundary::write_json(
        &root.join("validation_artifacts/observability/package-digest.json"),
        &json!({
            "schema": crate::cli::observe::types::RECEIPT_SCHEMA,
            "run_id": "run-durable-query-failure",
            "correlation_id": "corr-durable-query-failure",
            "candidate_digest": candidate,
            "operation": "package.digest",
            "status": "pass",
            "failure_class": "none",
            "why_failed": "none",
            "where_failed": "none",
            "next_repair": "none",
            "claim_impact": "source_package_digest_only",
            "event": {
                "run_id": "run-durable-query-failure",
                "correlation_id": "corr-durable-query-failure",
                "candidate_digest": candidate,
                "operation": "package.digest",
                "status": "pass",
                "failure_class": "none",
                "why_failed": "none",
                "where_failed": "none",
                "next_repair": "none",
                "claim_impact": "source_package_digest_only"
            }
        }),
    )
    .expect("product receipt");
    crate::json_boundary::write_json(
        &root.join("validation_artifacts/observability/live-loop/commands/traces-query.json"),
        &json!({
            "schema": crate::cli::observe::types::QUERY_SCHEMA,
            "query_kind": "traces",
            "status": "fail",
            "candidate_digest": candidate,
            "run_id": "run-durable-query-failure",
            "correlation_id": "corr-durable-query-failure",
            "trace_id": "trace-durable-query-failure",
            "failure_class": "observability_trace_tree_unavailable",
            "why_failed": "victoriatraces trace lookup returned 404 before the span tree was queryable",
            "where_failed": "observe.traces.query.live_backend",
            "next_repair": "rerun observe traces query by run and correlation",
            "claim_impact": "observability_reconciliation_blocked",
            "row_count": 0,
            "timeout_ms": 3000,
            "metric_task_count": 0,
            "metric_queue_depth": 0,
            "result_digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
        }),
    )
    .expect("failed durable query receipt");
    let command = crate::cli::observe::parse(&[
        "observe".to_string(),
        "explain-failure".to_string(),
        "--run-id".to_string(),
        "run-durable-query-failure".to_string(),
    ])
    .expect("parse")
    .expect("observe");

    let receipt = super::super::run(&root, &command).expect("explain");

    assert_eq!(receipt["observed_run"]["operation"], "observe.traces.query");
    assert_eq!(receipt["observed_status"], "fail");
    assert_eq!(
        receipt["failure_class"],
        "observability_trace_tree_unavailable"
    );
    assert_eq!(receipt["status"], "fail");
    assert!(
        receipt["explanation"]["root_cause"]
            .as_str()
            .expect("root cause")
            .contains("observability_trace_tree_unavailable")
    );
    assert!(
        receipt["explanation"]["query_evidence"]["traces"]["path"]
            .as_str()
            .expect("query receipt path")
            .contains("traces-query.json")
    );
    fs::remove_dir_all(root).expect("cleanup durable query failure");
}
