use crate::cli::observe;
use serde_json::json;
use std::fs;

#[test]
fn observe_run_covers_stack_query_explain_and_receipt_outputs() {
    let root = super::minimal_root("observe-run-edges");
    let receipts = root.join("validation_artifacts/observability/test");
    fs::create_dir_all(&receipts).expect("receipts");

    for (raw, expected) in [
        (&["observe", "stack", "up"][..], Some(1)),
        (&["observe", "stack", "down"][..], Some(1)),
        (&["observe", "stack", "gc", "plan"][..], Some(0)),
        (&["observe", "stack", "gc", "dry-run"][..], Some(0)),
        (&["observe", "stack", "gc", "apply"][..], Some(0)),
        (
            &["observe", "stack", "health", "--timeout-ms", "1"][..],
            None,
        ),
        (
            &["observe", "stack", "smoke", "--timeout-ms", "1"][..],
            None,
        ),
        (&["observe", "logs", "query", "--timeout-ms", "1"][..], None),
        (
            &["observe", "metrics", "query", "--timeout-ms", "1"][..],
            None,
        ),
        (
            &["observe", "traces", "query", "--timeout-ms", "1"][..],
            None,
        ),
        (
            &["observe", "explain-failure", "--run-id", "run-test"][..],
            Some(1),
        ),
        (
            &["observe", "explain-claim", "--claim-id", "claim-test"][..],
            Some(1),
        ),
        (
            &["observe", "explain-check", "--check-id", "check-test"][..],
            Some(1),
        ),
        (
            &["observe", "explain-law", "--law-id", "law-test"][..],
            Some(1),
        ),
    ] {
        let receipt_rel = format!(
            "validation_artifacts/observability/test/{}.json",
            raw.join("-")
        );
        let receipt = root.join(&receipt_rel);
        let mut raw_args = super::args(raw);
        raw_args.extend(["--receipt".to_string(), receipt_rel]);
        let command = observe::parse(&raw_args)
            .expect("parse")
            .expect("observe command");
        let code = observe::run(&root, &command).expect("run observe");
        if let Some(expected) = expected {
            assert_eq!(code, expected, "{raw_args:?}");
        }
        assert!(receipt.is_file(), "missing {}", receipt.display());
    }
    assert!(
        observe::parse(&super::args(&["package", "digest"]))
            .unwrap()
            .is_none()
    );
    let metrics_run = command(&["observe", "metrics", "query", "--run-id", "run-abc"]);
    let metrics_query = observe::query::query_text(&metrics_run);
    assert!(metrics_query.starts_with("sum by (__name__,operation,status,check_id"));
    assert!(metrics_query.contains("max_over_time({__name__=~"));
    assert!(metrics_query.contains("ultragoal_command_duration_ms"));
    assert!(!metrics_query.contains("run_id=\"run-abc\""));
    let escaped_metrics = command(&["observe", "metrics", "query", "--check-id", "check\\\"x\ny"]);
    let escaped_query = observe::query::query_text(&escaped_metrics);
    assert!(escaped_query.contains(r#"check_id="check\\"#));
    assert!(escaped_query.contains(r#"\""#));
    assert!(escaped_query.contains(r#"\n"#));
    assert!(!escaped_query.contains('\n'));
    assert!(!escaped_query.contains("run_id=\"check"));
    let traces_run = command(&["observe", "traces", "query", "--run-id", "run-abc"]);
    assert_eq!(
        observe::query::query_text(&traces_run),
        "{\"run_id\":\"run-abc\"}"
    );
    let metric_line = observe::telemetry::exporter_metric_line_for_test(&json!({
        "metric_name": "ultragoal_command_total",
        "metric_value": 1,
        "labels": {"operation": "observe.stack.smoke"},
        "run_id": "run-abc",
        "correlation_id": "corr-abc"
    }));
    assert!(metric_line.contains("operation=\"observe.stack.smoke\""));
    assert!(!metric_line.contains("run_id=\"run-abc\""));
    assert!(!metric_line.contains("correlation_id=\"corr-abc\""));
    assert!(observe::parse(&super::args(&["observe", "unknown"])).is_err());
    let default_receipt = observe::parse(&super::args(&["observe", "prove"]))
        .expect("parse")
        .expect("observe command");
    assert_eq!(default_receipt.timeout_ms, 30_000);
    assert_eq!(
        observe::run(&root, &default_receipt).expect("default run"),
        1
    );
    assert!(root.join(default_receipt.operation.receipt_rel()).is_file());
    let bad_receipt = observe::parse(&super::args(&[
        "observe",
        "stack",
        "gc",
        "plan",
        "--receipt",
        "owned.txt/receipt.json",
    ]))
    .expect("parse")
    .expect("observe");
    assert!(
        observe::run(&root, &bad_receipt)
            .expect_err("receipt write error")
            .contains("owned.txt")
    );
    assert_eq!(observe::csv_for_test(&json!({"not_array": true})), "none");
    fs::remove_dir_all(root).expect("cleanup observe run");
}

fn command(raw: &[&str]) -> observe::types::ObserveCommand {
    observe::parse(&super::args(raw))
        .expect("parse")
        .expect("observe command")
}
