use super::super::process::CommandOutput;
use super::super::reconciliation;
use crate::audit::observability::specs;
use serde_json::{Value, json};

const LOG_BODY: &str =
    r#"{"exporter":"victorialogs","_stream_id":"stream-1","_time":"2026-07-08T00:00:00Z"}"#;
const METRIC_BODY: &str = r#"{"status":"success","data":{"result":[{"metric":{"exporter":"victoriametrics"},"value":[1,"1"]}]}}"#;
const TRACE_BODY: &str = r#"{"data":[{"processes":{"p1":{}},"spans":[{"spanID":"s1"}]}]}"#;

#[test]
fn command_receipt_must_match_canonical_spec_operation_and_path() {
    let production = CommandOutput {
        status_success: true,
        exit_code: 0,
        stdout: "ultragoal-package-digest pass".to_string(),
        stderr: String::new(),
    };
    let receipt = receipt();
    let logs = query(LOG_BODY);
    let metrics = query(METRIC_BODY);
    let traces = query(TRACE_BODY);
    let explain = explain();
    let spec = specs::command("package digest").expect("package digest spec");

    let mut wrong_operation = receipt.clone();
    wrong_operation["operation"] = json!("source.audit");
    let report = reconciliation::report_for_spec(
        &production,
        &wrong_operation,
        &logs,
        &metrics,
        &traces,
        &explain,
        spec,
    );
    assert!(
        report
            .failures
            .contains(&"command_receipt_operation_mismatch".to_string())
    );

    let mut wrong_path = receipt.clone();
    wrong_path["receipt_path"] = json!("validation_artifacts/observability/other.json");
    let report = reconciliation::report_for_spec(
        &production,
        &wrong_path,
        &logs,
        &metrics,
        &traces,
        &explain,
        spec,
    );
    assert!(
        report
            .failures
            .contains(&"command_receipt_path_mismatch".to_string())
    );
}

fn receipt() -> Value {
    json!({
        "status": "pass",
        "candidate_digest": "sha256:current",
        "run_id": "run-current",
        "correlation_id": "corr-current",
        "operation": "package.digest",
        "receipt_path": "validation_artifacts/observability/package-digest.json"
    })
}

fn query(body: &str) -> Value {
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

fn explain() -> Value {
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
