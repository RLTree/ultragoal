use super::checks;
use super::process::CommandOutput;
use super::reconciliation::ReconciliationReport;
use crate::audit::observability::specs::CommandObservabilitySpec;
use serde_json::{Value, json};

pub(super) fn build(
    spec: CommandObservabilitySpec,
    production: &CommandOutput,
    command_receipt: &Value,
    queries: [&Value; 4],
    reconciliation: &ReconciliationReport,
    observable: bool,
) -> Value {
    json!({
        "command_id": spec.id,
        "family": spec.family,
        "operation": spec.operation,
        "roundtrip_status": super::roundtrip_status(observable),
        "candidate_digest": command_receipt.get("candidate_digest"),
        "run_id": command_receipt.get("run_id"),
        "correlation_id": command_receipt.get("correlation_id"),
        "production_exit_status": production.exit_code,
        "production_stdout": production.stdout,
        "production_stderr": production.stderr,
        "receipt_path": spec.receipt_rel,
        "artifact_path": command_receipt.get("artifact_path"),
        "receipt_artifact_reconciliation": reconciliation_summary(reconciliation),
        "failure_class": failure_class(reconciliation, observable),
        "why_failed": why_failed(reconciliation, observable),
        "where_failed": where_failed(reconciliation, observable),
        "next_repair": next_repair(reconciliation, observable),
        "validator_check_id": spec.validator_check_id,
        "query_roundtrip_paths": checks::query_paths(spec),
        "explain_roundtrip_path": checks::roundtrip_path(spec, "explain-failure"),
        "stdout_receipt_same_candidate": reconciliation.same_candidate,
        "logs_query_status": checks::status(queries[0]),
        "metrics_query_status": checks::status(queries[1]),
        "traces_query_status": checks::status(queries[2]),
        "explain_status": checks::status(queries[3]),
        "claim_name": "source-local command telemetry roundtrip claim",
        "product_behavior_observed": format!("real ultragoal command run: {}", spec.command_args.join(" ")),
        "proof_surface": "production stdout, command receipt, logs query receipt, metrics query receipt, traces query receipt, and explain receipt",
        "independent_reconciliation_surface": "same-candidate run/correlation/digest reconciliation across stdout, receipt, logs, metrics, traces, and explain output",
        "claim_status": if observable { "supported_source_local" } else { "partial_no_claim" },
        "claim_impact": spec.claim_impact
    })
}

fn reconciliation_summary(reconciliation: &ReconciliationReport) -> Value {
    json!({
        "status": if reconciliation.is_reconciled() { "pass" } else { "fail" },
        "failure_classes": reconciliation.failures,
        "rejects_receipt_existence_only": true,
        "rejects_generated_rows_only": true,
        "rejects_local_spool_only": true,
        "command_identity_reconciled_by": ["spec_command_id", "operation"],
        "compares_fields": [
            "spec_command_id",
            "candidate_digest",
            "run_id",
            "correlation_id",
            "operation",
            "receipt_path",
            "artifact_path"
        ]
    })
}

fn failure_class(reconciliation: &ReconciliationReport, observable: bool) -> String {
    if observable {
        "none".to_string()
    } else {
        reconciliation
            .failures
            .first()
            .cloned()
            .unwrap_or_else(|| "roundtrip_partial_without_reconciled_failure_class".to_string())
    }
}

fn why_failed(reconciliation: &ReconciliationReport, observable: bool) -> String {
    if observable {
        "none".to_string()
    } else if reconciliation.failures.is_empty() {
        "command roundtrip is partial but no reconciliation failure class was emitted".to_string()
    } else {
        format!(
            "command roundtrip withheld claim because reconciliation failed: {}",
            reconciliation.failures.join(",")
        )
    }
}

fn where_failed(reconciliation: &ReconciliationReport, observable: bool) -> &'static str {
    if observable {
        "none"
    } else if reconciliation.failures.is_empty() {
        "observe.command-roundtrip"
    } else {
        "observe.command-roundtrip.receipt_artifact_reconciliation"
    }
}

fn next_repair(reconciliation: &ReconciliationReport, observable: bool) -> &'static str {
    if observable {
        "none"
    } else if reconciliation.failures.is_empty() {
        "rerun command-roundtrip with live query receipts and inspect partial row statuses"
    } else {
        "repair the named reconciliation failure class, then rerun command-roundtrip for the same candidate"
    }
}
