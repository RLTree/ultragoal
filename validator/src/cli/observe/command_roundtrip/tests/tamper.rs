use super::super::process::CommandOutput;
use super::super::reconciliation;
use serde_json::json;

const LOG_BODY: &str =
    r#"{"exporter":"victorialogs","_stream_id":"stream-1","_time":"2026-07-08T00:00:00Z"}"#;
const METRIC_BODY: &str = r#"{"status":"success","data":{"result":[{"metric":{"exporter":"victoriametrics"},"value":[1,"1"]}]}}"#;
const TRACE_BODY: &str = r#"{"data":[{"processes":{"p1":{}},"spans":[{"spanID":"s1"}]}]}"#;

#[test]
fn reconciliation_report_fails_closed_for_proxy_and_tamper_classes() {
    let candidate = "sha256:current";
    let production = CommandOutput {
        status_success: true,
        exit_code: 0,
        stdout: "ultragoal-package-digest pass".to_string(),
        stderr: String::new(),
    };
    let receipt = json!({
        "status": "pass",
        "candidate_digest": candidate,
        "run_id": "run-current",
        "correlation_id": "corr-current",
        "command_id": "package digest",
        "operation": "package.digest",
        "receipt_path": "validation_artifacts/observability/package-digest.json"
    });
    let live = json!({
        "status": "pass",
        "row_count": 1,
        "candidate_digest": candidate,
        "run_id": "run-current",
        "correlation_id": "corr-current",
        "command_id": "package digest",
        "operation": "package.digest",
        "receipt_path": "validation_artifacts/observability/package-digest.json",
        "proof_surface": "live backend query",
        "rows": [{"body": LOG_BODY}]
    });
    let explain = json!({
        "status": "pass",
        "candidate_digest": candidate,
        "run_id": "run-current",
        "correlation_id": "corr-current",
        "explanation": {"query_evidence": {"logs": {"status": "pass"}}},
        "explanation_target": {
            "artifact_path": "plugin-manifest-draft.json",
            "receipt_path": "validation_artifacts/observability/package-digest.json"
        },
        "command_id": "package digest",
        "operation": "package.digest",
        "receipt_path": "validation_artifacts/observability/package-digest-explain-failure.json"
    });

    let mut metrics = live.clone();
    metrics["rows"] = json!([{"body": METRIC_BODY}]);
    let mut traces = live.clone();
    traces["rows"] = json!([{"body": TRACE_BODY}]);

    let ok = reconciliation::report(&production, &receipt, &live, &metrics, &traces, &explain);
    assert!(ok.is_reconciled(), "{ok:?}");

    let mut wrong_digest = live.clone();
    wrong_digest["candidate_digest"] = json!("sha256:old");
    let report = reconciliation::report(
        &production,
        &receipt,
        &wrong_digest,
        &metrics,
        &traces,
        &explain,
    );
    assert!(
        report
            .failures
            .contains(&"stale_or_wrong_digest_telemetry".to_string())
    );

    let mut wrong_run = live.clone();
    wrong_run["run_id"] = json!("run-old");
    let report = reconciliation::report(
        &production,
        &receipt,
        &wrong_run,
        &metrics,
        &traces,
        &explain,
    );
    assert!(
        report
            .failures
            .contains(&"logs_run_id_mismatch".to_string())
    );

    let mut local_spool = live.clone();
    local_spool["rows"] = json!([{"body":"{\"exporter\":\"victorialogs\"}"}]);
    let report = reconciliation::report(
        &production,
        &receipt,
        &local_spool,
        &metrics,
        &traces,
        &explain,
    );
    assert!(
        report
            .failures
            .contains(&"logs_local_spool_or_empty_proof".to_string())
    );

    let mut mismatched_artifact = live.clone();
    mismatched_artifact["observed_record"] =
        json!({"receipt_path":"validation_artifacts/other.json"});
    let report = reconciliation::report(
        &production,
        &receipt,
        &mismatched_artifact,
        &metrics,
        &traces,
        &explain,
    );
    assert!(
        report
            .failures
            .contains(&"receipt_artifact_field_mismatch".to_string())
    );

    let leaked = CommandOutput {
        stdout: "/Users/tree/private-token".to_string(),
        status_success: true,
        exit_code: 0,
        stderr: String::new(),
    };
    let report = reconciliation::report(&leaked, &receipt, &live, &metrics, &traces, &explain);
    assert!(
        report
            .failures
            .contains(&"unredacted_private_path_or_secret".to_string())
    );

    let mut metric_with_bad_label = metrics.clone();
    metric_with_bad_label["labels"] = json!({"run_id": "run-current"});
    let report = reconciliation::report(
        &production,
        &receipt,
        &live,
        &metric_with_bad_label,
        &traces,
        &explain,
    );
    assert!(
        report
            .failures
            .contains(&"metric_label_cardinality_violation".to_string())
    );

    let empty_product = CommandOutput {
        stdout: String::new(),
        stderr: String::new(),
        status_success: true,
        exit_code: 0,
    };
    let report =
        reconciliation::report(&empty_product, &receipt, &live, &metrics, &traces, &explain);
    assert!(
        report
            .failures
            .contains(&"receipt_exists_without_product_behavior".to_string())
    );
}

#[test]
fn reconciliation_accepts_live_backend_rows_and_target_receipt_fields() {
    let candidate = "sha256:current";
    let production = CommandOutput {
        status_success: true,
        exit_code: 0,
        stdout: "ultragoal-package-digest pass".to_string(),
        stderr: String::new(),
    };
    let receipt = json!({
        "status": "pass",
        "candidate_digest": candidate,
        "run_id": "run-current",
        "correlation_id": "corr-current",
        "operation": "package.digest",
        "receipt_path": "validation_artifacts/observability/package-digest.json",
        "artifact_path": "plugin-manifest-draft.json"
    });
    let query = json!({
        "status": "pass",
        "row_count": 1,
        "candidate_digest": candidate,
        "run_id": "run-current",
        "correlation_id": "corr-current",
        "observed_record": {
            "run_id": "run-current",
            "correlation_id": "corr-current",
            "candidate_digest": candidate,
            "operation": "package.digest"
        },
        "rows": [{"body": LOG_BODY}]
    });
    let metrics = json!({
        "status": "pass",
        "row_count": 1,
        "candidate_digest": candidate,
        "run_id": "run-current",
        "correlation_id": "corr-current",
        "rows": [{"body": METRIC_BODY}]
    });
    let traces = json!({
        "status": "pass",
        "row_count": 1,
        "candidate_digest": candidate,
        "run_id": "run-current",
        "correlation_id": "corr-current",
        "rows": [{"body": TRACE_BODY}]
    });
    let explain = json!({
        "status": "pass",
        "candidate_digest": candidate,
        "run_id": "run-current",
        "correlation_id": "corr-current",
        "operation": "observe.explain-failure",
        "receipt_path": "validation_artifacts/observability/command-roundtrip/package-digest-explain-failure.json",
        "explanation": {"query_evidence": {"logs": {"status": "pass"}}},
        "explanation_target": {
            "receipt_path": "validation_artifacts/observability/package-digest.json",
            "artifact_path": "plugin-manifest-draft.json"
        }
    });

    let report = reconciliation::report(&production, &receipt, &query, &metrics, &traces, &explain);
    assert!(report.is_reconciled(), "{report:?}");
}
