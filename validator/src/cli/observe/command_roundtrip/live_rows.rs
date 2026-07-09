use serde_json::Value;

pub(super) fn row_names_live_backend(surface: &str, row: &Value) -> bool {
    let Some(body) = row.get("body").and_then(Value::as_str) else {
        return false;
    };
    let Ok(parsed) = serde_json::from_str::<Value>(body) else {
        return false;
    };
    match surface {
        "logs" => is_victoria_logs_event(&parsed),
        "metrics" => is_victoria_metrics_vector(&parsed),
        "traces" => is_trace_payload(&parsed),
        _ => false,
    }
}

fn is_victoria_logs_event(value: &Value) -> bool {
    value.get("exporter").and_then(Value::as_str) == Some("victorialogs")
        && value.get("_stream_id").and_then(Value::as_str).is_some()
        && value.get("_time").and_then(Value::as_str).is_some()
}

fn is_victoria_metrics_vector(value: &Value) -> bool {
    value.get("status").and_then(Value::as_str) == Some("success")
        && value
            .pointer("/data/result")
            .and_then(Value::as_array)
            .is_some_and(|rows| rows.iter().any(is_victoria_metrics_result))
}

fn is_victoria_metrics_result(value: &Value) -> bool {
    value
        .get("metric")
        .and_then(|metric| metric.get("exporter"))
        .and_then(Value::as_str)
        == Some("victoriametrics")
        && value.get("value").and_then(Value::as_array).is_some()
}

fn is_trace_payload(value: &Value) -> bool {
    value
        .get("data")
        .and_then(Value::as_array)
        .is_some_and(|traces| traces.iter().any(trace_has_process_and_span))
}

fn trace_has_process_and_span(value: &Value) -> bool {
    value.get("processes").and_then(Value::as_object).is_some()
        && value
            .get("spans")
            .and_then(Value::as_array)
            .is_some_and(|spans| !spans.is_empty())
}
