use super::*;
use crate::cli::observe::command::{ObserveCommand, ObserveOperation};
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
        Some(std::path::Path::new(
            "validation_artifacts/observability/explain.json",
        )),
    );
}

#[test]
fn stdout_failure_hint_uses_observed_product_operation_for_explain_targets() {
    let value = json!({
        "status": "fail",
        "observed_operation": "loop.run"
    });

    assert_eq!(
        query_hint::failure_metric_operation(&command(ObserveOperation::ExplainFailure), &value),
        "loop.run"
    );

    let nested = json!({
        "status": "fail",
        "observed_run": {
            "operation": "coverage.prove"
        }
    });
    assert_eq!(
        query_hint::failure_metric_operation(&command(ObserveOperation::ExplainFailure), &nested),
        "coverage.prove"
    );

    assert_eq!(
        query_hint::failure_metric_operation(
            &command(ObserveOperation::ExplainFailure),
            &json!({})
        ),
        "observe.explain-failure"
    );
    assert_eq!(
        query_hint::failure_metric_operation(
            &command(ObserveOperation::MetricsQuery),
            &json!({"observed_operation":"none"})
        ),
        "observe.metrics.query"
    );

    let query = query_hint::failure_metric_query(
        &command(ObserveOperation::ExplainFailure),
        &json!({
            "observed_operation": "loop.run",
            "observed_status": "blocked"
        }),
    );
    assert!(query.contains("operation=\"loop.run\""));
    assert!(query.contains("status=\"blocked\""));
}

#[test]
fn stdout_command_roundtrip_line_names_product_and_reconciliation_surfaces() {
    let value = json!({
        "claim_ceiling": "source-local observability command roundtrip only",
        "results": [{
            "command_id": "package digest",
            "product_behavior_observed": "real ultragoal command run: package digest",
            "proof_surface": "production stdout, command receipt, logs query receipt, metrics query receipt, traces query receipt, and explain receipt",
            "independent_reconciliation_surface": "same-candidate run/correlation/digest reconciliation across stdout, receipt, logs, metrics, traces, and explain output",
            "candidate_digest": "sha256:current",
            "run_id": "run-command",
            "correlation_id": "corr-command",
            "receipt_path": "validation_artifacts/observability/package-digest.json",
            "artifact_path": "plugin-manifest-draft.json",
            "failure_class": "none",
            "why_failed": "none",
            "where_failed": "none",
            "next_repair": "none",
            "claim_status": "supported_source_local"
        }]
    });
    let lines = roundtrip::lines(&value);
    assert_eq!(lines.len(), 1);
    let line = &lines[0];
    for needle in [
        "command_id=package digest",
        "product_behavior='real ultragoal command run: package digest'",
        "proof_surface='production stdout, command receipt",
        "independent_reconciliation_surface='same-candidate run/correlation/digest",
        "candidate=sha256:current",
        "run_id=run-command",
        "correlation_id=corr-command",
        "command_receipt=validation_artifacts/observability/package-digest.json",
        "artifact_path=plugin-manifest-draft.json",
        "failure_class=none",
        "where_failed=none",
        "claim_status=supported_source_local",
        "claim_ceiling='source-local observability command roundtrip only'",
    ] {
        assert!(line.contains(needle), "{line}");
    }
}

#[test]
fn stdout_command_roundtrip_line_reports_each_result_row() {
    let value = json!({
        "claim_ceiling": "source-local observability command roundtrip only",
        "results": [
            {"command_id": "package digest", "claim_status": "supported_source_local"},
            {"command_id": "install audit", "claim_status": "partial_no_claim"}
        ]
    });
    let lines = roundtrip::lines(&value);
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("command_id=package digest"));
    assert!(lines[1].contains("command_id=install audit"));
    assert!(lines[1].contains("claim_status=partial_no_claim"));
}
