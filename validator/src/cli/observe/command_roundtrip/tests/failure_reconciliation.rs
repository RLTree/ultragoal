use super::super::checks::is_command_observable;
use super::super::process::CommandOutput;
use serde_json::{Value, json};

const LOG_BODY: &str =
    r#"{"exporter":"victorialogs","_stream_id":"stream-1","_time":"2026-07-08T00:00:00Z"}"#;
const METRIC_BODY: &str = r#"{"status":"success","data":{"result":[{"metric":{"exporter":"victoriametrics"},"value":[1,"1"]}]}}"#;
const TRACE_BODY: &str = r#"{"data":[{"processes":{"p1":{}},"spans":[{"spanID":"s1"}]}]}"#;

#[test]
fn failed_command_roundtrip_is_observable_when_agent_legible_and_same_candidate() {
    let production = failed_production();
    let receipt = command_receipt();
    let logs = live_rows(LOG_BODY, true);
    let metrics = live_rows(METRIC_BODY, false);
    let traces = live_rows(TRACE_BODY, false);

    assert!(is_command_observable(
        &production,
        &receipt,
        &logs,
        &metrics,
        &traces,
        &logs
    ));

    let direct_logs = direct_failure_rows(LOG_BODY, true);
    let direct_traces = direct_failure_rows(TRACE_BODY, false);
    assert!(is_command_observable(
        &production,
        &receipt,
        &direct_logs,
        &metrics,
        &direct_traces,
        &direct_logs
    ));

    let opaque_logs = json!({
        "status": "pass",
        "row_count": 1,
        "candidate_digest": "sha256:current",
        "run_id": "run-install",
        "correlation_id": "corr-install",
        "operation": "install_audit",
        "receipt_path": "validation_artifacts/cli/install-audit-receipt.json",
        "rows": [{"body": LOG_BODY}],
        "observed_why_failed": "unknown",
        "observed_where_failed": "package_surface_audit#/failures/0",
        "observed_next_repair": "refresh installed package after source-local proof graph passes"
    });
    assert!(!is_command_observable(
        &production,
        &receipt,
        &opaque_logs,
        &metrics,
        &direct_traces,
        &logs
    ));

    let opaque_stdout = CommandOutput {
        stdout: "failed; inspect receipt".to_string(),
        ..production
    };
    assert!(!is_command_observable(
        &opaque_stdout,
        &receipt,
        &logs,
        &metrics,
        &traces,
        &logs
    ));
}

fn failed_production() -> CommandOutput {
    CommandOutput {
        status_success: false,
        exit_code: 1,
        stdout: [
            "ultragoal-surface-audit fail",
            "run_id=run-install",
            "correlation_id=corr-install",
            "failed_check=install-audit-observability-binding",
            "why=package_surface_digest_mismatch",
            "where=package_surface_audit#/failures/0",
            "next_repair=refresh installed package after source-local proof graph passes",
            "query_logs='ultragoal observe logs query --run-id run-install'",
            "query_metrics='ultragoal observe metrics query --run-id run-install'",
            "query_traces='ultragoal observe traces query --run-id run-install'",
        ]
        .join(" "),
        stderr: String::new(),
    }
}

fn command_receipt() -> Value {
    json!({
        "status": "fail",
        "candidate_digest": "sha256:current",
        "run_id": "run-install",
        "correlation_id": "corr-install",
        "operation": "install_audit",
        "receipt_path": "validation_artifacts/cli/install-audit-receipt.json",
        "why_failed": "package_surface_digest_mismatch",
        "where_failed": "package_surface_audit#/failures/0",
        "next_repair": "refresh installed package after source-local proof graph passes",
        "claim_impact": "blocks_install_cache_parity_readiness_release_completion_update_goal"
    })
}

fn live_rows(body: &str, explain: bool) -> Value {
    let mut value = direct_failure_rows(body, explain);
    value["observed_why_failed"] = json!("package_surface_digest_mismatch");
    value["observed_where_failed"] = json!("package_surface_audit#/failures/0");
    value["observed_next_repair"] =
        json!("refresh installed package after source-local proof graph passes");
    value
}

fn direct_failure_rows(body: &str, explain: bool) -> Value {
    let mut value = json!({
        "status": "pass",
        "row_count": 1,
        "candidate_digest": "sha256:current",
        "run_id": "run-install",
        "correlation_id": "corr-install",
        "operation": "install_audit",
        "receipt_path": "validation_artifacts/cli/install-audit-receipt.json",
        "rows": [{"body": body}],
        "why_failed": "package_surface_digest_mismatch",
        "where_failed": "package_surface_audit#/failures/0",
        "next_repair": "refresh installed package after source-local proof graph passes"
    });
    if explain {
        value["explanation"] = json!({"query_evidence": {"logs": {"status": "pass"}}});
    }
    value
}
