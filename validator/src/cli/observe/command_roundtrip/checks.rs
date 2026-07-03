use super::process::CommandOutput;
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
    production.status_success
        && command_receipt.get("status").and_then(Value::as_str) == Some("pass")
        && query_passed(logs)
        && query_passed(metrics)
        && query_passed(traces)
        && explain.get("status").and_then(Value::as_str) == Some("pass")
        && same_candidate(command_receipt, logs, metrics, traces, explain)
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
