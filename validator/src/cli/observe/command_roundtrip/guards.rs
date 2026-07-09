use super::process::CommandOutput;
use serde_json::Value;

pub(super) fn private_or_secret_leak(
    production: &CommandOutput,
    command_receipt: &Value,
    logs: &Value,
    metrics: &Value,
    traces: &Value,
    explain: &Value,
) -> bool {
    let mut haystacks = vec![production.stdout.as_str(), production.stderr.as_str()];
    for value in [command_receipt, logs, metrics, traces, explain] {
        if contains_sensitive_text(value) {
            return true;
        }
    }
    haystacks.drain(..).any(|text| {
        sensitive_needles()
            .iter()
            .any(|needle| text.contains(needle))
    })
}

pub(super) fn query_bounds_or_redaction_failed(values: &[&Value]) -> bool {
    values.iter().any(|value| {
        field_failed(value, "bounded_output_status")
            || field_failed(value, "bounded_output_proof")
            || field_failed(value, "redaction_status")
            || field_failed(value, "redaction_proof")
    })
}

pub(super) fn metric_label_cardinality_violation(metrics: &Value) -> bool {
    let labels = metrics
        .get("labels")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|items| items.iter());
    labels.into_iter().any(|(key, value)| {
        let key = key.as_str();
        let raw = value.as_str().unwrap_or_default();
        matches!(
            key,
            "run_id"
                | "correlation_id"
                | "trace_id"
                | "span_id"
                | "candidate_digest"
                | "receipt_path"
                | "artifact_path"
                | "private_path"
                | "prompt_hash"
        ) || raw.len() > 128
            || raw.contains("/Users/")
            || raw.contains("/private/tmp/")
    })
}

fn contains_sensitive_text(value: &Value) -> bool {
    match value {
        Value::String(text) => sensitive_needles()
            .iter()
            .any(|needle| text.contains(needle)),
        Value::Array(items) => items.iter().any(contains_sensitive_text),
        Value::Object(items) => items.values().any(contains_sensitive_text),
        _ => false,
    }
}

fn sensitive_needles() -> &'static [&'static str] {
    &["/Users/", "/private/tmp/", "sk-", "token=", "secret="]
}

fn field_failed(value: &Value, field: &str) -> bool {
    value.get(field).and_then(Value::as_str) == Some("fail")
}
