use serde_json::{Value, json};

pub(super) fn summary(query_kind: &str, rows: &[Value]) -> Value {
    if query_kind != "metrics" {
        return json!({});
    }
    let mut operation = "unknown".to_string();
    let mut failure_class = "none".to_string();
    let mut saturation_status = "unknown".to_string();
    let mut high_cardinality_labels = "pass";
    let mut traffic_count = 0_u64;
    let mut task_count = 0_u64;
    let mut latency_ms = 0_u64;
    let mut error_count = 0_u64;
    let mut queue_depth = 0_u64;
    let mut latest_sample_unix = 0_i64;
    for sample in metric_samples(rows) {
        let labels = sample.get("metric").unwrap_or(&Value::Null);
        if operation == "unknown" {
            operation = metric_label(labels, "operation", "unknown").to_string();
        }
        let sample_failure = metric_label(labels, "failure_class", "none");
        if failure_class == "none" && sample_failure != "none" {
            failure_class = sample_failure.to_string();
        }
        if saturation_status == "unknown" {
            saturation_status = metric_label(labels, "saturation_status", "unknown").to_string();
        }
        if has_high_cardinality_label(labels) {
            high_cardinality_labels = "fail";
        }
        latest_sample_unix = latest_sample_unix.max(prom_timestamp(&sample));
        let value = prom_value(&sample);
        match metric_label(labels, "__name__", "ultragoal_command_total") {
            "ultragoal_command_duration_ms" => latency_ms = latency_ms.max(value),
            "ultragoal_command_task_count" => task_count += value,
            "ultragoal_command_queue_depth" => {
                if value >= queue_depth {
                    queue_depth = value;
                    saturation_status =
                        metric_label(labels, "saturation_status", "unknown").to_string();
                }
            }
            _ => {
                traffic_count += value;
                if metric_label(labels, "status", "pass") != "pass" || sample_failure != "none" {
                    error_count += value;
                }
            }
        }
    }
    json!({
        "operation": operation,
        "traffic_task_count": task_count.max(traffic_count),
        "traffic_count": traffic_count,
        "task_count": task_count,
        "latency_ms": latency_ms,
        "error_count": error_count,
        "failure_class": failure_class,
        "queue_depth": queue_depth,
        "latest_sample_unix": latest_sample_unix,
        "saturation_status": format!("{saturation_status};queue_depth={queue_depth}"),
        "high_cardinality_labels": high_cardinality_labels
    })
}

pub(super) fn text<'a>(value: &'a Value, field: &str, default: &'a str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or(default)
}

fn metric_samples(rows: &[Value]) -> Vec<Value> {
    let mut out = Vec::new();
    for row in rows {
        let Some(body) = row.get("body").and_then(Value::as_str) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(body) else {
            continue;
        };
        if let Some(samples) = value
            .get("data")
            .and_then(|data| data.get("result"))
            .and_then(Value::as_array)
        {
            out.extend(samples.iter().cloned());
        }
    }
    out
}

fn prom_value(sample: &Value) -> u64 {
    sample
        .get("value")
        .and_then(Value::as_array)
        .and_then(|items| items.get(1))
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<f64>().ok())
        .map(|value| value.max(0.0).round() as u64)
        .unwrap_or(0)
}

fn prom_timestamp(sample: &Value) -> i64 {
    sample
        .get("value")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_f64().map(|value| value.round() as i64))
                .or_else(|| value.as_str().and_then(|value| value.parse::<i64>().ok()))
        })
        .unwrap_or(0)
}

fn metric_label<'a>(labels: &'a Value, field: &str, default: &'a str) -> &'a str {
    labels.get(field).and_then(Value::as_str).unwrap_or(default)
}

fn has_high_cardinality_label(labels: &Value) -> bool {
    [
        "run_id",
        "correlation_id",
        "trace_id",
        "span_id",
        "candidate_digest",
        "receipt_path",
        "artifact_path",
    ]
    .iter()
    .any(|key| labels.get(*key).is_some())
}
