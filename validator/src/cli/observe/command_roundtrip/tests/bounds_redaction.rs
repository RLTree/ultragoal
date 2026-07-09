use super::super::process::CommandOutput;
use super::super::reconciliation;
use serde_json::{Value, json};

const LOG_BODY: &str =
    r#"{"exporter":"victorialogs","_stream_id":"stream-1","_time":"2026-07-08T00:00:00Z"}"#;
const METRIC_BODY: &str = r#"{"status":"success","data":{"result":[{"metric":{"exporter":"victoriametrics"},"value":[1,"1"]}]}}"#;
const TRACE_BODY: &str = r#"{"data":[{"processes":{"p1":{}},"spans":[{"spanID":"s1"}]}]}"#;

#[test]
fn reconciliation_fails_closed_for_unbounded_or_unredacted_query_receipts() {
    let production = CommandOutput {
        status_success: true,
        exit_code: 0,
        stdout: "ultragoal-package-digest pass".to_string(),
        stderr: String::new(),
    };
    let receipt = command_receipt();
    let logs = query_receipt(LOG_BODY);
    let metrics = query_receipt(METRIC_BODY);
    let traces = query_receipt(TRACE_BODY);
    let explain = explain_receipt();

    let mut unbounded_logs = logs.clone();
    unbounded_logs["bounded_output_status"] = json!("fail");
    assert_bounds_redaction_failure(
        &production,
        &receipt,
        &unbounded_logs,
        &metrics,
        &traces,
        &explain,
    );

    let mut failed_redaction = explain.clone();
    failed_redaction["redaction_proof"] = json!("fail");
    assert_bounds_redaction_failure(
        &production,
        &receipt,
        &logs,
        &metrics,
        &traces,
        &failed_redaction,
    );
}

fn assert_bounds_redaction_failure(
    production: &CommandOutput,
    receipt: &Value,
    logs: &Value,
    metrics: &Value,
    traces: &Value,
    explain: &Value,
) {
    let report = reconciliation::report(production, receipt, logs, metrics, traces, explain);
    assert!(
        report
            .failures
            .contains(&"query_bounds_or_redaction_failed".to_string()),
        "{report:?}"
    );
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
        "bounded_output_status": "pass",
        "redaction_status": "pass",
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
        },
        "bounded_output_proof": "pass",
        "redaction_proof": "pass"
    })
}
