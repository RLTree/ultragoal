use super::{process::CommandOutput, reconciliation};
use crate::audit::observability::specs::CommandObservabilitySpec;
use serde_json::Value;
use std::path::PathBuf;

pub(super) fn query_paths(spec: CommandObservabilitySpec) -> Vec<PathBuf> {
    ["logs", "metrics", "traces"]
        .into_iter()
        .map(|kind| roundtrip_path(spec, kind))
        .collect()
}

pub(super) fn roundtrip_path(spec: CommandObservabilitySpec, suffix: &str) -> PathBuf {
    PathBuf::from(format!(
        "validation_artifacts/observability/command-roundtrip/{}-{suffix}.json",
        spec.id.replace(' ', "-")
    ))
}

pub(super) fn same_candidate(
    values: &Value,
    logs: &Value,
    metrics: &Value,
    traces: &Value,
    explain: &Value,
) -> bool {
    let Some(candidate) = values.get("candidate_digest").and_then(Value::as_str) else {
        return false;
    };
    [logs, metrics, traces, explain]
        .into_iter()
        .all(|value| value.get("candidate_digest").and_then(Value::as_str) == Some(candidate))
}

pub(super) fn is_command_observable(
    production: &CommandOutput,
    command_receipt: &Value,
    logs: &Value,
    metrics: &Value,
    traces: &Value,
    explain: &Value,
) -> bool {
    let report =
        reconciliation::report(production, command_receipt, logs, metrics, traces, explain);
    query_passed(logs)
        && query_passed(metrics)
        && query_passed(traces)
        && explain.get("status").and_then(Value::as_str) == Some("pass")
        && report.is_reconciled()
        && production_and_receipt_are_legible(production, command_receipt, logs, traces, explain)
}

fn production_and_receipt_are_legible(
    production: &CommandOutput,
    command_receipt: &Value,
    logs: &Value,
    traces: &Value,
    explain: &Value,
) -> bool {
    match command_receipt.get("status").and_then(Value::as_str) {
        Some("pass") => production.status_success,
        Some("fail") => {
            !production.status_success
                && failed_stdout_is_agent_legible(&production.stdout)
                && failed_receipt_is_agent_legible(command_receipt)
                && observed_failure_is_agent_legible(logs)
                && observed_failure_is_agent_legible(traces)
                && observed_failure_is_agent_legible(explain)
        }
        _ => false,
    }
}

fn failed_stdout_is_agent_legible(stdout: &str) -> bool {
    [
        "run_id=",
        "correlation_id=",
        "failed_check=",
        "why=",
        "where=",
        "next_repair=",
        "query_logs=",
        "query_metrics=",
        "query_traces=",
    ]
    .into_iter()
    .all(|needle| stdout.contains(needle))
}

fn failed_receipt_is_agent_legible(value: &Value) -> bool {
    [
        "run_id",
        "correlation_id",
        "candidate_digest",
        "why_failed",
        "where_failed",
        "next_repair",
        "claim_impact",
    ]
    .into_iter()
    .all(|field| meaningful_text(value, field))
}

fn observed_failure_is_agent_legible(value: &Value) -> bool {
    let why = value
        .get("observed_why_failed")
        .or_else(|| value.get("why_failed"));
    let where_failed = value
        .get("observed_where_failed")
        .or_else(|| value.get("where_failed"));
    let next_repair = value
        .get("observed_next_repair")
        .or_else(|| value.get("next_repair"));
    meaningful_observed_text(why)
        && meaningful_observed_text(where_failed)
        && meaningful_observed_text(next_repair)
}

pub(super) fn query_passed(value: &Value) -> bool {
    value.get("status").and_then(Value::as_str) == Some("pass")
        && value
            .get("row_count")
            .and_then(Value::as_u64)
            .is_some_and(|count| count > 0)
}

pub(super) fn status(value: &Value) -> &str {
    value
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("missing")
}

pub(super) fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("observability command roundtrip missing {key}"))
}

fn meaningful_text(value: &Value, field: &str) -> bool {
    value
        .get(field)
        .and_then(Value::as_str)
        .is_some_and(|text| !weak_text(text))
}

fn meaningful_observed_text(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_str)
        .is_some_and(|text| !weak_text(text))
}

fn weak_text(text: &str) -> bool {
    let normalized = text.trim().to_ascii_lowercase();
    normalized.is_empty()
        || matches!(normalized.as_str(), "none" | "unknown")
        || normalized.contains("validation failed")
        || normalized.contains("inspect receipt")
}
