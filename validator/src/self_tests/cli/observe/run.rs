use crate::cli::observe;
use serde_json::json;
use std::fs;

#[test]
fn observe_run_covers_stack_query_explain_and_receipt_outputs() {
    let root = super::minimal_root("observe-run-edges");
    let receipts = root.join("receipts");
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
            Some(0),
        ),
        (
            &["observe", "explain-claim", "--claim-id", "claim-test"][..],
            Some(0),
        ),
        (
            &["observe", "explain-check", "--check-id", "check-test"][..],
            Some(0),
        ),
        (
            &["observe", "explain-law", "--law-id", "law-test"][..],
            Some(0),
        ),
    ] {
        let receipt = receipts.join(format!("{}.json", raw.join("-")));
        let mut raw_args = super::args(raw);
        raw_args.extend(["--receipt".to_string(), receipt.display().to_string()]);
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
    assert_eq!(
        observe::query::query_text(&metrics_run),
        "ultragoal_command_total{run_id=\"run-abc\"}"
    );
    let escaped_metrics = command(&["observe", "metrics", "query", "--run-id", "run\\\"x\ny"]);
    let escaped_query = observe::query::query_text(&escaped_metrics);
    assert!(escaped_query.contains(r#"run\\"#));
    assert!(escaped_query.contains(r#"\""#));
    assert!(escaped_query.contains(r#"\n"#));
    assert!(!escaped_query.contains('\n'));
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
    assert!(metric_line.contains("run_id=\"run-abc\""));
    assert!(metric_line.contains("correlation_id=\"corr-abc\""));
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
    let health_command = command(&["observe", "stack", "health"]);
    let health_pass = observe::stack::health_receipt(
        &root,
        &health_command,
        vec![json!({"service":"victorialogs","status":"pass"})],
    )
    .expect("health pass");
    assert_eq!(health_pass["status"], "pass");
    let health_fail = observe::stack::health_receipt(
        &root,
        &health_command,
        vec![json!({"service":"victorialogs","status":"fail"})],
    )
    .expect("health fail");
    assert_eq!(health_fail["status"], "fail");
    assert!(
        health_fail["why_failed"]
            .as_str()
            .unwrap()
            .contains("health")
    );
    let compose_unhealthy = observe::stack::compose_health_row_for_test(
        r#"{"Service":"otel-collector","Name":"otel","State":"running","Health":"unhealthy"}"#,
    );
    assert_eq!(compose_unhealthy["status"], "fail");
    assert_eq!(
        observe::stack::compose_health_row_for_test("not json")["failure"],
        "docker compose ps emitted malformed json"
    );
    assert_eq!(
        observe::stack::compose_output_error_for_test().unwrap_err(),
        "docker compose ps unavailable"
    );
    assert_eq!(
        observe::stack::compose_health_rows_from_result_for_test(Err("missing docker".into()))[0]["failure"],
        "missing docker"
    );
    assert_eq!(
        observe::stack::compose_health_rows_from_result_for_test(Ok((false, String::new())))[0]["failure"],
        "docker compose ps returned nonzero"
    );
    assert_eq!(
        observe::stack::compose_health_rows_from_result_for_test(Ok((true, String::new())))[0]["failure"],
        "docker compose ps returned no services"
    );
    let parsed_rows = observe::stack::compose_health_rows_from_result_for_test(Ok((
        true,
        r#"{"Service":"vector","Name":"vector","State":"running","Health":"healthy"}"#.into(),
    )));
    assert_eq!(parsed_rows[0]["status"], "pass");
    let endpoint_pass_compose_fail = observe::stack::health_receipt(
        &root,
        &health_command,
        vec![
            json!({"service":"otel-collector","check_source":"http_endpoint","status":"pass"}),
            compose_unhealthy,
        ],
    )
    .expect("health fail on compose health");
    assert_eq!(endpoint_pass_compose_fail["status"], "fail");
    assert_eq!(
        observe::stack::compose_health_row_for_test(
            r#"{"Service":"otel-collector","Name":"otel","State":"running","Health":"healthy"}"#
        )["status"],
        "pass"
    );

    let smoke_command = command(&["observe", "stack", "smoke"]);
    let candidate = crate::package::inventory::package_digest(&root).expect("digest");
    let smoke_pass =
        observe::stack::smoke_receipt(&root, &smoke_command, &candidate, true, true, true)
            .expect("smoke pass");
    assert_eq!(smoke_pass["status"], "pass");
    let smoke_fail =
        observe::stack::smoke_receipt(&root, &smoke_command, &candidate, true, false, true)
            .expect("smoke fail");
    assert_eq!(smoke_fail["status"], "fail");
    assert!(smoke_fail["why_failed"].as_str().unwrap().contains("smoke"));
    let missing_manifest_root =
        crate::self_tests::boundaries::support::temp_root("observe-bad-root");
    fs::create_dir_all(&missing_manifest_root).expect("missing manifest root");
    assert!(
        observe::stack::health_receipt(&missing_manifest_root, &health_command, vec![])
            .unwrap_err()
            .contains("plugin-manifest-draft.json")
    );
    assert!(
        observe::stack::smoke_receipt(
            &missing_manifest_root,
            &smoke_command,
            "sha256:bad",
            true,
            true,
            true,
        )
        .unwrap_err()
        .contains("plugin-manifest-draft.json")
    );
    let gc_error = observe::run(
        &missing_manifest_root,
        &command(&["observe", "stack", "gc", "plan"]),
    )
    .expect_err("stack dispatch error");
    assert!(gc_error.contains("plugin-manifest-draft.json"));
    let query_error = observe::run(
        &missing_manifest_root,
        &command(&["observe", "logs", "query", "--timeout-ms", "1"]),
    )
    .expect_err("query dispatch error");
    assert!(query_error.contains("plugin-manifest-draft.json"));
    let prove_error = observe::run(&missing_manifest_root, &command(&["observe", "prove"]))
        .expect_err("prove dispatch error");
    assert!(prove_error.contains("plugin-manifest-draft.json"));
    let explain_error = observe::run(
        &missing_manifest_root,
        &command(&["observe", "explain-law", "--law-id", "law"]),
    )
    .expect_err("explain dispatch error");
    assert!(explain_error.contains("plugin-manifest-draft.json"));
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
    fs::remove_dir_all(missing_manifest_root).expect("cleanup bad observe root");
    fs::remove_dir_all(root).expect("cleanup observe run");
}

fn command(raw: &[&str]) -> observe::types::ObserveCommand {
    observe::parse(&super::args(raw))
        .expect("parse")
        .expect("observe command")
}
