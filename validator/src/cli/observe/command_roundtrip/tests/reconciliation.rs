use super::super::checks::{is_command_observable, query_passed, same_candidate};
use super::super::process::CommandOutput;
use serde_json::json;

const LOG_BODY: &str =
    r#"{"exporter":"victorialogs","_stream_id":"stream-1","_time":"2026-07-08T00:00:00Z"}"#;
const METRIC_BODY: &str = r#"{"status":"success","data":{"result":[{"metric":{"exporter":"victoriametrics"},"value":[1,"1"]}]}}"#;
const TRACE_BODY: &str = r#"{"data":[{"processes":{"p1":{}},"spans":[{"spanID":"s1"}]}]}"#;

#[test]
fn query_and_candidate_reconciliation_are_strict() {
    let candidate = "sha256:fit";
    let command_receipt = json!({"candidate_digest": candidate});
    let pass_query = json!({
        "status": "pass",
        "row_count": 1,
        "candidate_digest": candidate,
        "run_id": "run-fit",
        "correlation_id": "corr-fit",
        "proof_surface": "live backend query",
        "rows": [{"body": LOG_BODY}]
    });
    let pass_metrics = json!({
        "status": "pass",
        "row_count": 1,
        "candidate_digest": candidate,
        "run_id": "run-fit",
        "correlation_id": "corr-fit",
        "rows": [{"body": METRIC_BODY}]
    });
    let pass_traces = json!({
        "status": "pass",
        "row_count": 1,
        "candidate_digest": candidate,
        "run_id": "run-fit",
        "correlation_id": "corr-fit",
        "rows": [{"body": TRACE_BODY}]
    });
    let empty_query = json!({"status": "pass", "row_count": 0, "candidate_digest": candidate});
    let explain = json!({
        "status": "pass",
        "candidate_digest": candidate,
        "run_id": "run-fit",
        "correlation_id": "corr-fit",
        "proof_surface": "observe explain receipt",
        "explanation": {"query_evidence": {"logs": {"status": "pass"}}}
    });
    assert!(query_passed(&pass_query));
    assert!(!query_passed(&empty_query));
    assert!(same_candidate(
        &command_receipt,
        &pass_query,
        &pass_metrics,
        &pass_traces,
        &explain
    ));
    assert!(!same_candidate(
        &json!({}),
        &pass_query,
        &pass_metrics,
        &pass_traces,
        &explain
    ));
    let wrong = json!({"status": "pass", "row_count": 1, "candidate_digest": "sha256:old"});
    assert!(!same_candidate(
        &command_receipt,
        &wrong,
        &pass_metrics,
        &pass_traces,
        &explain
    ));
    let production = CommandOutput {
        status_success: true,
        exit_code: 0,
        stdout: "ok".to_string(),
        stderr: String::new(),
    };
    assert!(is_command_observable(
        &production,
        &json!({
            "status": "pass",
            "candidate_digest": candidate,
            "run_id": "run-fit",
            "correlation_id": "corr-fit",
            "operation": "package.digest",
            "receipt_path": "validation_artifacts/observability/package-digest.json"
        }),
        &pass_query,
        &pass_metrics,
        &pass_traces,
        &explain
    ));
    let failed_production = CommandOutput {
        status_success: false,
        exit_code: 1,
        stdout: String::new(),
        stderr: "failed".to_string(),
    };
    assert!(!is_command_observable(
        &failed_production,
        &json!({
            "status": "pass",
            "candidate_digest": candidate,
            "run_id": "run-fit",
            "correlation_id": "corr-fit",
            "operation": "package.digest",
            "receipt_path": "validation_artifacts/observability/package-digest.json"
        }),
        &pass_query,
        &pass_metrics,
        &pass_traces,
        &explain
    ));
    assert!(!is_command_observable(
        &production,
        &json!({
            "status": "unknown",
            "candidate_digest": candidate,
            "run_id": "run-fit",
            "correlation_id": "corr-fit",
            "operation": "package.digest",
            "receipt_path": "validation_artifacts/observability/package-digest.json"
        }),
        &pass_query,
        &pass_metrics,
        &pass_traces,
        &explain
    ));
}
