use super::*;
use crate::cli::observe::types::{ObserveCommand, ObserveOperation};
use serde_json::json;

fn command(operation: ObserveOperation) -> ObserveCommand {
    ObserveCommand {
        operation,
        receipt: None,
        query: None,
        run_id: Some("run-stdout".to_string()),
        correlation_id: Some("corr-stdout".to_string()),
        claim_id: None,
        check_id: None,
        law_id: None,
        target_command: None,
        target_family: None,
        row_limit: 100,
        byte_limit: 4096,
        timeout_ms: 1000,
    }
}

#[test]
fn stdout_query_contracts_report_bounded_match_and_failure_context() {
    let pass = json!({"status":"pass","row_count":2});
    assert!(query_matched(&pass));
    assert_eq!(query_status(Some(&json!({"status":"pass"}))), "pass");
    assert_eq!(query_status(None), "missing");

    let fail = json!({
        "status": "fail",
        "why_failed": "coverage receipt target digest mismatch requiring exact coverage rerun"
    });
    let status = query_status(Some(&fail));
    assert!(status.starts_with("fail(coverage receipt target"));
    assert_eq!(clip("abcdef", 3), "abc...");

    let no_rows = json!({"status":"pass","row_count":0});
    assert!(!query_matched(&no_rows));
}

#[test]
fn stdout_field_rendering_handles_explain_paths_and_claim_fallbacks() {
    assert_eq!(
        field_string(&json!({"flag":true}), "flag", "missing"),
        "true"
    );
    assert_eq!(field_string(&json!({"n":7}), "n", "missing"), "7");
    assert_eq!(number_string(&json!({}), "missing_count"), "0");
    assert_eq!(
        array_csv(Some(&json!([
            "validation_artifacts/coverage/coverage-receipt.json"
        ]))),
        "validation_artifacts/coverage/coverage-receipt.json"
    );
    assert_eq!(array_csv(Some(&json!([]))), "none");
    assert_eq!(
        claim_impact(&json!({"event":{"claim_impact":"event_claim_ceiling"}})),
        "event_claim_ceiling"
    );

    let explanation = json!({
        "query_evidence": {
            "logs": {"status":"pass"},
            "metrics": {"status":"fail","why_failed":"metric failure"},
            "traces": {"status":"missing"}
        }
    });
    assert_eq!(
        query_summary(&explanation),
        "logs:pass,metrics:fail(metric failure),traces:missing"
    );
}

#[test]
fn stdout_failure_hint_uses_command_specific_query_contract() {
    let value = json!({
        "status": "fail",
        "candidate_digest": "sha256:test",
        "run_id": "run-stdout",
        "correlation_id": "corr-stdout",
        "trace_id": "trace-stdout",
        "failure_class": "coverage_prove_failure",
        "why_failed": "coverage receipt is stale",
        "where_failed": "coverage.manifest_digest",
        "next_repair": "rerun exact coverage",
        "claim_impact": "coverage_blocks_readiness"
    });
    print_failure(
        &command(ObserveOperation::ExplainFailure),
        &value,
        std::path::Path::new("validation_artifacts/observability/explain.json"),
    );
}
