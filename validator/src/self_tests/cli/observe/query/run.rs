use crate::cli::observe;
use crate::cli::observe::query::QueryKind;
use std::fs;
use std::process::Command;

#[test]
fn observe_query_run_covers_pass_retry_and_failure_paths() {
    let root = super::super::minimal_root("observe-query-run");
    let command = super::command(&["observe", "logs", "query", "--timeout-ms", "1"]);
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

    let missing_candidate = observe::query::result_from_output(
        &root,
        &command,
        QueryKind::Logs,
        "*".to_string(),
        Ok("abcdef".to_string()),
    )
    .expect("missing candidate output");
    assert_eq!(missing_candidate["status"], "fail");
    assert_eq!(
        missing_candidate["failure"],
        "observability_query_candidate_digest_missing"
    );
    let current_candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let same_candidate = observe::query::result_from_output(
        &root,
        &command,
        QueryKind::Logs,
        "*".to_string(),
        Ok(format!(
            "{{\"candidate_digest\":\"{current_candidate}\",\"echo\":\"{current_candidate}\",\"row\":1}}"
        )),
    )
    .expect("same candidate output");
    assert_eq!(same_candidate["status"], "pass");
    let observed_failure = observe::query::result_from_output(
        &root,
        &command,
        QueryKind::Logs,
        "*".to_string(),
        Ok(format!(
            "{{\"candidate_digest\":\"{current_candidate}\",\"status\":\"fail\",\"failure_class\":\"observability_gate_failure\",\"why_failed\":\"live stack unhealthy\",\"where_failed\":\"observe.prove\",\"next_repair\":\"run observe stack health\",\"claim_impact\":\"update_goal_blocked\",\"law_id\":\"full-local-observability-stack-integration-non-opaque-failure\",\"check_id\":\"full-local-observability-stack-integration-non-opaque-failure\",\"claim_id\":\"gate-92-observability-control-plane\",\"run_id\":\"run-observed\",\"correlation_id\":\"corr-observed\"}}"
        )),
    )
    .expect("observed failure output");
    assert_eq!(observed_failure["status"], "pass");
    assert_eq!(
        observed_failure["observed_failure_class"],
        "observability_gate_failure"
    );
    assert_eq!(
        observed_failure["observed_why_failed"],
        "live stack unhealthy"
    );
    assert_eq!(observed_failure["observed_where_failed"], "observe.prove");
    assert_eq!(
        observed_failure["observed_next_repair"],
        "run observe stack health"
    );
    assert_eq!(
        observed_failure["observed_claim_impact"],
        "update_goal_blocked"
    );
    let metric_observed_failure = observe::query::result_from_output(
        &root,
        &command,
        QueryKind::Metrics,
        "*".to_string(),
        Ok(format!(
            "{{\"status\":\"success\",\"data\":{{\"result\":[{{\"metric\":{{\"candidate_digest\":\"{current_candidate}\",\"status\":\"fail\",\"failure_class\":\"observability_gate_failure\",\"operation\":\"observe.prove\",\"claim_impact\":\"update_goal_blocked\"}}}}]}}}}"
        )),
    )
    .expect("metric observed failure output");
    assert_eq!(
        metric_observed_failure["observed_failure_class"],
        "observability_gate_failure"
    );
    assert_eq!(
        metric_observed_failure["observed_why_failed"],
        "bounded metric signal carries failure class only; query logs and traces for repair-specific why_failed"
    );
    let trace_observed_failure = observe::query::result_from_output(
        &root,
        &command,
        QueryKind::Traces,
        "*".to_string(),
        Ok(format!(
            "{{\"data\":[{{\"processes\":{{\"p1\":{{\"tags\":[{{\"key\":\"candidate_digest\",\"value\":\"{current_candidate}\"}}]}}}},\"spans\":[{{\"tags\":[{{\"key\":\"status\",\"value\":\"fail\"}},{{\"key\":\"failure_class\",\"value\":\"observability_gate_failure\"}},{{\"key\":\"why_failed\",\"value\":\"live stack unhealthy\"}},{{\"key\":\"operation\",\"value\":\"observe.prove\"}},{{\"key\":\"claim_impact\",\"value\":\"update_goal_blocked\"}}]}}]}}]}}"
        )),
    )
    .expect("trace observed failure output");
    assert_eq!(trace_observed_failure["status"], "pass");
    assert_eq!(
        trace_observed_failure["observed_why_failed"],
        "live stack unhealthy"
    );
    let explanatory_stale_digest = observe::query::result_from_output(
        &root,
        &command,
        QueryKind::Logs,
        "*".to_string(),
        Ok(format!(
            "{{\"candidate_digest\":\"{current_candidate}\",\"why_failed\":\"dependency was stale: {}\"}}",
            crate::self_tests::boundaries::support::sha('e')
        )),
    )
    .expect("explanatory stale digest");
    assert_eq!(explanatory_stale_digest["status"], "pass");
    let nested_row_candidate = observe::query::result_from_output(
        &root,
        &command,
        QueryKind::Logs,
        "*".to_string(),
        Ok(format!(
            "{{\"rows\":[{{\"body\":\"{{\\\"candidate_digest\\\":\\\"{}\\\",\\\"why_failed\\\":\\\"old {}\\\"}}\"}}]}}",
            current_candidate,
            crate::self_tests::boundaries::support::sha('d')
        )),
    )
    .expect("nested row candidate");
    assert_eq!(nested_row_candidate["status"], "pass");
    let truncated_candidate = observe::query::result_from_output(
        &root,
        &command,
        QueryKind::Logs,
        "*".to_string(),
        Ok("{\"candidate_digest\":\"sha256:short\",\"row\":1}".to_string()),
    )
    .expect("truncated candidate output");
    assert_eq!(truncated_candidate["status"], "fail");
    assert_eq!(
        truncated_candidate["failure"],
        "observability_query_candidate_digest_missing"
    );
    let stale_candidate = observe::query::result_from_output(
        &root,
        &command,
        QueryKind::Logs,
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
        QueryKind::Logs,
        "*".to_string(),
        Err("curl query failed".to_string()),
    )
    .expect("fail output");
    assert_eq!(fail["status"], "fail");
    assert_eq!(fail["why_failed"], "curl query failed");
    assert_eq!(fail["where_failed"], "observe.logs.query");
    assert_eq!(
        fail["next_repair"],
        "run stack health and smoke, then rerun bounded query"
    );

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
    assert!(
        observe::telemetry::exporter_launch_failure_for_test().starts_with("curl launch failed:")
    );
    fs::remove_dir_all(root).expect("cleanup query run");
}
