use crate::cli::observe;
use serde_json::json;
use std::fs;

#[test]
fn observe_stack_receipts_report_health_smoke_and_dispatch_boundaries() {
    let root = super::minimal_root("observe-stack-receipts");
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
        crate::self_tests::boundaries::workspace_fixtures::temp_root("observe-bad-root");
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

    fs::remove_dir_all(missing_manifest_root).expect("cleanup bad observe root");
    fs::remove_dir_all(root).expect("cleanup observe stack receipts");
}

fn command(raw: &[&str]) -> observe::command::ObserveCommand {
    observe::parse(&super::args(raw))
        .expect("parse")
        .expect("observe command")
}
