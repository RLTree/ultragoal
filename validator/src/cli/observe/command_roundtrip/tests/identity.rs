use super::super::process::CommandOutput;
use super::super::reconciliation;
use serde_json::{Value, json};

const LOG_BODY: &str =
    r#"{"exporter":"victorialogs","_stream_id":"stream-1","_time":"2026-07-08T00:00:00Z"}"#;
const METRIC_BODY: &str = r#"{"status":"success","data":{"result":[{"metric":{"exporter":"victoriametrics"},"value":[1,"1"]}]}}"#;
const TRACE_BODY: &str = r#"{"data":[{"processes":{"p1":{}},"spans":[{"spanID":"s1"}]}]}"#;

#[test]
fn reconciliation_fails_closed_when_command_receipt_identity_is_missing() {
    let mut receipt = command_receipt();
    receipt.as_object_mut().expect("receipt").remove("run_id");
    let report = reconciliation::report(
        &production(),
        &receipt,
        &query_receipt(LOG_BODY),
        &query_receipt(METRIC_BODY),
        &query_receipt(TRACE_BODY),
        &explain_receipt(),
    );
    assert!(
        report
            .failures
            .contains(&"command_receipt_identity_missing".to_string()),
        "{report:?}"
    );
}

fn production() -> CommandOutput {
    CommandOutput {
        status_success: true,
        exit_code: 0,
        stdout: "ultragoal-package-digest pass".to_string(),
        stderr: String::new(),
    }
}

fn command_receipt() -> Value {
    json!({
        "status": "pass",
        "candidate_digest": "sha256:current",
        "run_id": "run-current",
        "correlation_id": "corr-current",
        "operation": "package.digest",
        "receipt_path": "validation_artifacts/observability/package-digest.json"
    })
}

fn query_receipt(body: &str) -> Value {
    json!({
        "status": "pass",
        "row_count": 1,
        "candidate_digest": "sha256:current",
        "run_id": "run-current",
        "correlation_id": "corr-current",
        "operation": "package.digest",
        "receipt_path": "validation_artifacts/observability/package-digest.json",
        "rows": [{"body": body}]
    })
}

fn explain_receipt() -> Value {
    json!({
        "status": "pass",
        "candidate_digest": "sha256:current",
        "run_id": "run-current",
        "correlation_id": "corr-current",
        "explanation": {"query_evidence": {"logs": {"status": "pass"}}},
        "explanation_target": {
            "receipt_path": "validation_artifacts/observability/package-digest.json"
        }
    })
}
