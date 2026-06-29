use serde_json::{Value, json};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn emit(event: &Value, metric: &Value, trace: &Value) {
    let _ = post_json(
        "http://127.0.0.1:9428/insert/jsonline?_stream_fields=stream,command,law_id,check_id,claim_id,candidate_digest&_msg_field=message&_time_field=timestamp",
        event,
    );
    let _ = post_text(
        "http://127.0.0.1:8428/api/v1/import/prometheus",
        &metric_line(metric),
    );
    let _ = post_json(
        "http://127.0.0.1:10428/insert/opentelemetry/v1/traces",
        &trace_payload(trace),
    );
}

fn post_json(url: &str, value: &Value) -> Result<(), String> {
    let body = serde_json::to_string(value).expect("serde_json::Value serialization is infallible");
    post(url, &body, "application/json")
}

fn post_text(url: &str, body: &str) -> Result<(), String> {
    post(url, body, "text/plain")
}

fn post(url: &str, body: &str, content_type: &str) -> Result<(), String> {
    let output = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--max-time",
            "5",
            "-H",
            &format!("Content-Type: {content_type}"),
            "-X",
            "POST",
            url,
            "--data-binary",
            body,
        ])
        .output();
    post_output_result(output)?;
    Ok(())
}

fn post_output_result(output: Result<std::process::Output, std::io::Error>) -> Result<(), String> {
    let output = output.map_err(|err| format!("curl launch failed: {err}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "curl export failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn metric_line(metric: &Value) -> String {
    let name = metric
        .get("metric_name")
        .and_then(Value::as_str)
        .unwrap_or("ultragoal_command_total");
    let mut labels = metric
        .get("labels")
        .and_then(Value::as_object)
        .map(|items| {
            items
                .iter()
                .filter_map(|(key, value)| value.as_str().map(|raw| (key.as_str(), raw)))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for key in ["run_id", "correlation_id", "trace_id", "span_id"] {
        if let Some(raw) = metric.get(key).and_then(Value::as_str) {
            labels.push((key, raw));
        }
    }
    let labels = labels
        .into_iter()
        .map(|(key, raw)| format!("{key}=\"{}\"", label_value(raw)))
        .collect::<Vec<_>>()
        .join(",");
    let value = metric
        .get("metric_value")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    format!("{name}{{{labels}}} {value}\n")
}

fn trace_payload(trace: &Value) -> Value {
    let trace_id = hex_id(trace, "trace_id", 32);
    let span_id = hex_id(trace, "span_id", 16);
    let now = now_nanos();
    json!({
        "resourceSpans": [{
            "resource": {"attributes": [
                attr("service.name", "ultragoal"),
                attr("run_id", str_field(trace, "run_id")),
                attr("candidate_digest", str_field(trace, "candidate_digest"))
            ]},
            "scopeSpans": [{
                "scope": {"name": "ultragoal"},
                "spans": [{
                    "traceId": trace_id,
                    "spanId": span_id,
                    "name": str_field(trace, "operation"),
                    "kind": 1,
                    "startTimeUnixNano": now.to_string(),
                    "endTimeUnixNano": (now + 1_000_000).to_string(),
                    "attributes": [
                        attr("run_id", str_field(trace, "run_id")),
                        attr("correlation_id", str_field(trace, "correlation_id")),
                        attr("operation", str_field(trace, "operation")),
                        attr("status", str_field(trace, "status")),
                        attr("law_id", str_field(trace, "law_id")),
                        attr("check_id", str_field(trace, "check_id")),
                        attr("claim_id", str_field(trace, "claim_id")),
                        attr("failure_class", str_field(trace, "failure_class")),
                        attr("why_failed", str_field(trace, "why_failed")),
                        attr("next_repair", str_field(trace, "next_repair")),
                        attr("claim_impact", str_field(trace, "claim_impact"))
                    ]
                }]
            }]
        }]
    })
}

fn attr(key: &str, value: &str) -> Value {
    json!({"key": key, "value": {"stringValue": value}})
}

fn str_field<'a>(value: &'a Value, field: &str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or("")
}

fn hex_id(value: &Value, field: &str, len: usize) -> String {
    let raw = str_field(value, field);
    let digest = crate::digest::bytes(raw.as_bytes());
    digest
        .trim_start_matches("sha256:")
        .chars()
        .take(len)
        .collect()
}

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn label_value(raw: &str) -> String {
    raw.chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.'))
        .take(128)
        .collect()
}

#[cfg(test)]
pub(crate) fn failed_post_message_for_test() -> String {
    post(
        "http://127.0.0.1:9/ultragoal-test",
        "{}",
        "application/json",
    )
    .expect_err("closed local discard port should reject the exporter probe")
}

#[cfg(test)]
pub(crate) fn failed_launch_message_for_test() -> String {
    post_output_result(Err(std::io::Error::other("synthetic curl launch failure")))
        .expect_err("synthetic launch error")
}

#[cfg(test)]
pub(crate) fn metric_line_for_test(metric: &Value) -> String {
    metric_line(metric)
}
