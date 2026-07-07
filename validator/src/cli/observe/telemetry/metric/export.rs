use serde_json::Value;

pub(in crate::cli::observe::telemetry) fn lines(metric: &Value) -> String {
    if let Some(samples) = metric.get("samples").and_then(Value::as_array) {
        return samples.iter().map(line).collect::<String>();
    }
    line(metric)
}

fn line(metric: &Value) -> String {
    let name = metric
        .get("metric_name")
        .and_then(Value::as_str)
        .unwrap_or("ultragoal_command_total");
    let labels = metric
        .get("labels")
        .and_then(Value::as_object)
        .map(|items| {
            items
                .iter()
                .filter_map(|(key, value)| {
                    let key = key.as_str();
                    label_allowed(key).then(|| value.as_str().map(|raw| (key, raw)))?
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let labels = labels
        .into_iter()
        .map(|(key, raw)| format!("{key}=\"{}\"", label_value(raw)))
        .collect::<Vec<_>>()
        .join(",");
    let value = metric
        .get("metric_value")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    match sample_timestamp_ms(metric) {
        Some(timestamp) => format!("{name}{{{labels}}} {value} {timestamp}\n"),
        None => format!("{name}{{{labels}}} {value}\n"),
    }
}

fn label_allowed(key: &str) -> bool {
    matches!(
        key,
        "command"
            | "operation"
            | "status"
            | "law_id"
            | "check_id"
            | "claim_id"
            | "surface"
            | "failure_class"
            | "exporter"
            | "saturation_status"
    )
}

fn label_value(raw: &str) -> String {
    raw.chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.'))
        .take(128)
        .collect()
}

fn sample_timestamp_ms(metric: &Value) -> Option<i64> {
    metric
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(crate::audit::clock::parse_iso_seconds)
        .map(|seconds| seconds.saturating_mul(1_000))
}
