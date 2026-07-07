use super::*;
use serde_json::json;

#[test]
fn failure_reconciliation_names_missing_mismatched_and_opaque_observed_failures() {
    let event = json!({
        "status": "fail",
        "failure_class": "coverage_prove_failure"
    });
    assert_eq!(
        target_failure_mismatch(QueryKind::Logs, &event, None),
        Some("observability_logs_failure_missing:coverage_prove_failure".to_string())
    );

    let mismatched = json!({"failure_class": "source_audit_check_failure"});
    assert_eq!(
        target_failure_mismatch(QueryKind::Logs, &event, Some(&mismatched)),
        Some(
            "observability_logs_failure_mismatch:source_audit_check_failure!=coverage_prove_failure"
                .to_string()
        )
    );

    let opaque = json!({"failure_class": "coverage_prove_failure", "why_failed": "none"});
    assert_eq!(
        target_failure_mismatch(QueryKind::Traces, &event, Some(&opaque)),
        Some("observability_traces_why_failed_missing:coverage_prove_failure".to_string())
    );
}

#[test]
fn target_record_reconciliation_names_identity_mismatches() {
    let event = json!({
        "run_id": "run-target",
        "correlation_id": "corr-target",
        "candidate_digest": "sha256:current",
        "operation": "source.audit"
    });
    assert_eq!(
        target_record_failure(QueryKind::Logs, &event, &[]),
        Some("observability_logs_target_record_missing:run_id=run-target".to_string())
    );

    let wrong_operation = json!({
        "run_id": "run-target",
        "correlation_id": "corr-target",
        "candidate_digest": "sha256:current",
        "operation": "coverage.prove"
    });
    assert_eq!(
        target_record_failure(
            QueryKind::Logs,
            &event,
            &[json!({"body": wrong_operation.to_string()})]
        ),
        Some(
            "observability_logs_target_record_mismatch:operation:coverage.prove!=source.audit"
                .to_string()
        )
    );

    let wrong_correlation = json!({
        "run_id": "run-target",
        "correlation_id": "corr-other",
        "candidate_digest": "sha256:current",
        "operation": "source.audit"
    });
    assert_eq!(
        target_record_failure(
            QueryKind::Traces,
            &event,
            &[json!({"body": wrong_correlation.to_string()})]
        ),
        Some(
            "observability_traces_target_record_mismatch:correlation_id:corr-other!=corr-target"
                .to_string()
        )
    );

    let event_with_law = json!({
        "run_id": "run-target",
        "correlation_id": "corr-target",
        "candidate_digest": "sha256:current",
        "operation": "source.audit",
        "law_id": "law-target",
        "check_id": "check-target",
        "claim_id": "claim-target"
    });
    let missing_law = json!({
        "run_id": "run-target",
        "correlation_id": "corr-target",
        "candidate_digest": "sha256:current",
        "operation": "source.audit",
        "law_id": "none",
        "check_id": "check-target",
        "claim_id": "claim-target"
    });
    assert_eq!(
        target_record_failure(
            QueryKind::Traces,
            &event_with_law,
            &[json!({"body": missing_law.to_string()})]
        ),
        Some("observability_traces_target_record_mismatch:law_id:none!=law-target".to_string())
    );
}

#[test]
fn target_label_uses_check_id_or_unknown_when_run_id_is_absent() {
    assert_eq!(
        target_label(&json!({"check_id":"coverage-current-receipt"})),
        "check_id=coverage-current-receipt"
    );
    assert_eq!(target_label(&json!({})), "unknown-target");
}
