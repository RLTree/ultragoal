use super::process::CommandOutput;
use super::{checks, expected, guards, identity, live_rows};
use crate::audit::observability::specs::CommandObservabilitySpec;
use serde_json::Value;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ReconciliationReport {
    pub(super) same_candidate: bool,
    pub(super) failures: Vec<String>,
}

impl ReconciliationReport {
    pub(super) fn is_reconciled(&self) -> bool {
        self.failures.is_empty()
    }
}

pub(super) fn report(
    production: &CommandOutput,
    command_receipt: &Value,
    logs: &Value,
    metrics: &Value,
    traces: &Value,
    explain: &Value,
) -> ReconciliationReport {
    report_with_spec(
        production,
        command_receipt,
        logs,
        metrics,
        traces,
        explain,
        None,
    )
}

pub(super) fn report_for_spec(
    production: &CommandOutput,
    command_receipt: &Value,
    logs: &Value,
    metrics: &Value,
    traces: &Value,
    explain: &Value,
    spec: CommandObservabilitySpec,
) -> ReconciliationReport {
    report_with_spec(
        production,
        command_receipt,
        logs,
        metrics,
        traces,
        explain,
        Some(spec),
    )
}

fn report_with_spec(
    production: &CommandOutput,
    command_receipt: &Value,
    logs: &Value,
    metrics: &Value,
    traces: &Value,
    explain: &Value,
    spec: Option<CommandObservabilitySpec>,
) -> ReconciliationReport {
    let mut failures = Vec::new();
    let same_candidate = checks::same_candidate(command_receipt, logs, metrics, traces, explain);
    if identity::command_receipt_identity_missing(command_receipt) {
        failures.push("command_receipt_identity_missing".to_string());
    }
    if let Some(spec) = spec {
        failures.extend(expected::receipt_mismatches(command_receipt, spec));
    }
    if !same_candidate {
        failures.push("stale_or_wrong_digest_telemetry".to_string());
    }
    if let Some(failure) = run_correlation_failure(command_receipt, logs, metrics, traces, explain)
    {
        failures.push(failure);
    }
    for (surface, value) in [
        ("logs", logs),
        ("metrics", metrics),
        ("traces", traces),
        ("explain", explain),
    ] {
        if !query_or_explain_has_live_rows(surface, value) {
            failures.push(format!("{surface}_local_spool_or_empty_proof"));
        }
    }
    if receipt_exists_without_product_behavior(production, command_receipt) {
        failures.push("receipt_exists_without_product_behavior".to_string());
    }
    if receipt_or_artifact_field_mismatch(command_receipt, logs, metrics, traces, explain) {
        failures.push("receipt_artifact_field_mismatch".to_string());
    }
    if guards::private_or_secret_leak(production, command_receipt, logs, metrics, traces, explain) {
        failures.push("unredacted_private_path_or_secret".to_string());
    }
    if guards::query_bounds_or_redaction_failed(&[logs, metrics, traces, explain]) {
        failures.push("query_bounds_or_redaction_failed".to_string());
    }
    if guards::metric_label_cardinality_violation(metrics) {
        failures.push("metric_label_cardinality_violation".to_string());
    }
    ReconciliationReport {
        same_candidate,
        failures,
    }
}

fn run_correlation_failure(
    command_receipt: &Value,
    logs: &Value,
    metrics: &Value,
    traces: &Value,
    explain: &Value,
) -> Option<String> {
    for field in ["run_id", "correlation_id"] {
        let expected = command_receipt.get(field).and_then(Value::as_str)?;
        for (surface, value) in [
            ("logs", logs),
            ("metrics", metrics),
            ("traces", traces),
            ("explain", explain),
        ] {
            if observed_text(value, field) != Some(expected) {
                return Some(format!("{surface}_{field}_mismatch"));
            }
        }
    }
    None
}

fn query_or_explain_has_live_rows(surface: &str, value: &Value) -> bool {
    if surface == "explain" {
        return value.get("status").and_then(Value::as_str) == Some("pass")
            && value
                .get("explanation")
                .and_then(|explanation| explanation.get("query_evidence"))
                .is_some();
    }
    checks::query_passed(value)
        && value
            .get("rows")
            .and_then(Value::as_array)
            .is_some_and(|rows| {
                rows.iter()
                    .any(|row| live_rows::row_names_live_backend(surface, row))
            })
}

fn receipt_exists_without_product_behavior(production: &CommandOutput, receipt: &Value) -> bool {
    receipt.get("status").and_then(Value::as_str).is_some()
        && production.stdout.trim().is_empty()
        && production.stderr.trim().is_empty()
}

fn receipt_or_artifact_field_mismatch(
    command_receipt: &Value,
    logs: &Value,
    metrics: &Value,
    traces: &Value,
    explain: &Value,
) -> bool {
    for field in ["receipt_path", "artifact_path", "command_id", "operation"] {
        let Some(expected) = command_receipt.get(field).and_then(Value::as_str) else {
            continue;
        };
        for value in [logs, metrics, traces, explain] {
            if let Some(observed) = observed_text(value, field) {
                if observed != expected {
                    return true;
                }
            }
        }
    }
    false
}

fn observed_text<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    let target = value
        .get("observed_record")
        .and_then(|record| record.get(field))
        .and_then(Value::as_str)
        .or_else(|| {
            value
                .get("explanation_target")
                .and_then(|target| target.get(field))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            value
                .get("observed_run")
                .and_then(|run| run.get(field))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            value
                .get("target_record")
                .and_then(|record| record.get(field))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            value
                .get("event")
                .and_then(|event| event.get(field))
                .and_then(Value::as_str)
        });
    if matches!(
        field,
        "receipt_path" | "artifact_path" | "command_id" | "operation"
    ) {
        return target;
    }
    value.get(field).and_then(Value::as_str).or(target)
}
