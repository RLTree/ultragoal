use crate::cli::observe;
use crate::cli::observe::types::ObserveOperation;
use serde_json::json;
use std::fs;
use std::process::Command;

#[test]
fn observe_query_run_covers_pass_retry_and_failure_paths() {
    let root = super::super::minimal_root("observe-query-run");
    let not_query = observe::query::run(
        &root,
        &observe::types::ObserveCommand {
            operation: ObserveOperation::Prove,
            receipt: None,
            query: None,
            run_id: None,
            claim_id: None,
            check_id: None,
            law_id: None,
            row_limit: 1,
            byte_limit: 10,
            timeout_ms: 1,
        },
    )
    .expect("not query");
    assert_eq!(not_query["status"], "fail");
    assert_eq!(not_query["failure"], "not an observability query");

    let command = super::command(&["observe", "logs", "query"]);
    let matched = observe::query::retry_until_match_for_test(
        &command,
        vec![Ok("".to_string()), Ok("{\"row\":1}".to_string())],
    )
    .expect("matched retry");
    assert_eq!(matched, "{\"row\":1}");
    let retry_failure =
        observe::query::retry_until_match_for_test(&command, vec![Ok("".to_string())])
            .expect_err("retry failure");
    assert!(
        retry_failure == "observability query returned no matching rows"
            || retry_failure == "test outputs exhausted"
    );

    let pass = observe::query::result_from_output(
        &root,
        &command,
        "*".to_string(),
        Ok("abcdef".to_string()),
    )
    .expect("pass output");
    assert_eq!(pass["status"], "pass");
    let current_candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let same_candidate = observe::query::result_from_output(
        &root,
        &command,
        "*".to_string(),
        Ok(format!(
            "{{\"candidate_digest\":\"{current_candidate}\",\"echo\":\"{current_candidate}\",\"row\":1}}"
        )),
    )
    .expect("same candidate output");
    assert_eq!(same_candidate["status"], "pass");
    let truncated_candidate = observe::query::result_from_output(
        &root,
        &command,
        "*".to_string(),
        Ok("{\"candidate_digest\":\"sha256:short\",\"row\":1}".to_string()),
    )
    .expect("truncated candidate output");
    assert_eq!(truncated_candidate["status"], "pass");
    let stale_candidate = observe::query::result_from_output(
        &root,
        &command,
        "*".to_string(),
        Ok(format!(
            "{{\"candidate_digest\":\"{}\",\"row\":1}}",
            crate::self_tests::boundaries::support::sha('f')
        )),
    )
    .expect("stale candidate output");
    assert_eq!(stale_candidate["status"], "fail");
    assert!(
        stale_candidate["failure"]
            .as_str()
            .unwrap()
            .contains("observability_query_candidate_digest_mismatch")
    );
    let fail = observe::query::result_from_output(
        &root,
        &command,
        "*".to_string(),
        Err("curl query failed".to_string()),
    )
    .expect("fail output");
    assert_eq!(fail["status"], "fail");

    let ok_output = Command::new("printf")
        .arg("ok")
        .output()
        .expect("printf output");
    assert_eq!(observe::query::curl_output_body(ok_output).unwrap(), "ok");
    let bad_output = Command::new("false").output().expect("false output");
    assert!(
        observe::query::curl_output_body(bad_output)
            .unwrap_err()
            .starts_with("curl query failed:")
    );
    assert!(
        observe::query::curl_result_body(Err(std::io::Error::other(
            "synthetic curl launch failure",
        )))
        .unwrap_err()
        .starts_with("curl launch failed:")
    );
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
    assert!(
        observe::telemetry::exporter_launch_failure_for_test().starts_with("curl launch failed:")
    );
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
    fs::remove_dir_all(root).expect("cleanup query run");
}
