use crate::cli::observe::types::{ObserveCommand, ObserveOperation};
use serde_json::json;
use std::path::Path;

mod metrics;

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
