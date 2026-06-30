use crate::cli::observe;
use serde_json::json;
use std::fs;

#[test]
fn observe_query_receipt_and_spool_helpers_are_typed() {
    let root = super::super::minimal_root("observe-query-spool");
    let command = super::command(&["observe", "logs", "query"]);
    let query_receipt = observe::telemetry::query_result(
        &root,
        &command,
        "logs",
        "*".to_string(),
        vec![json!({"body":"row"})],
        "pass",
        None,
    )
    .expect("query receipt");
    assert_eq!(query_receipt["status"], "pass");
    assert_eq!(query_receipt["why_failed"], "none");
    assert_eq!(query_receipt["where_failed"], "none");
    assert_eq!(
        query_receipt["next_repair"],
        "keep receipt same-candidate and rerun source audit before any readiness claim"
    );
    assert_eq!(
        observe::telemetry::query_receipt_text_for_test(&query_receipt, "candidate_digest")
            .expect("candidate text"),
        query_receipt["candidate_digest"]
            .as_str()
            .expect("candidate")
    );
    for field in ["candidate_digest", "run_id", "correlation_id"] {
        let mut missing_field = query_receipt.clone();
        missing_field
            .as_object_mut()
            .expect("query receipt object")
            .remove(field);
        assert!(
            observe::telemetry::query_receipt_text_for_test(&missing_field, field)
                .expect_err("missing query telemetry field")
                .contains(field)
        );
    }
    let base_query_failure = observe::telemetry::base_receipt(
        &root,
        &super::command(&["observe", "logs", "query"]),
        "fail",
        Some("query failed"),
    )
    .expect("base query failure");
    assert_eq!(
        base_query_failure["next_repair"],
        "run stack health and smoke, then rerun bounded query"
    );
    let metric_line = observe::telemetry::exporter_metric_line_for_test(&json!({
        "metric_name": "ultragoal_command_total",
        "metric_value": 1.0,
        "labels": {"bad": "a/b secret=token ok"}
    }));
    assert!(metric_line.contains("absecrettokenok"));
    observe::telemetry::spool_write_for_test(&root, &json!({"event":"ok"})).expect("spool write");
    let spool_text =
        fs::read_to_string(root.join("validation_artifacts/observability/spool/events.jsonl"))
            .expect("spool events");
    assert!(spool_text.contains(r#""event":"ok""#));
    let blocked_root = root.join("not-a-directory-root");
    fs::write(&blocked_root, b"file").expect("blocked root file");
    assert!(
        observe::telemetry::spool_write_for_test(&blocked_root, &json!({"event":"fail"}))
            .expect_err("blocked spool root")
            .contains("validation_artifacts/observability/spool")
    );
    let blocked_event_root = super::super::minimal_root("observe-query-spool-open-error");
    fs::create_dir_all(
        blocked_event_root.join("validation_artifacts/observability/spool/events.jsonl"),
    )
    .expect("blocked event directory");
    assert!(
        observe::telemetry::spool_write_for_test(&blocked_event_root, &json!({"event":"fail"}))
            .expect_err("blocked event path")
            .contains("events.jsonl")
    );
    fs::remove_dir_all(blocked_event_root).expect("cleanup blocked event root");
    let read_only_path = root.join("read-only-events.jsonl");
    fs::write(&read_only_path, b"file").expect("read-only fixture file");
    let read_only_error =
        observe::telemetry::spool_write_line_read_only_failure_for_test(&read_only_path);
    assert!(read_only_error.contains("read-only-events.jsonl"));
    fs::remove_dir_all(root).expect("cleanup query spool");
}
