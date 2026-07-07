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
        &metric_lines(metric),
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
    post_output_result(output)
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

#[cfg(test)]
#[path = "exporter_tests.rs"]
mod tests;

fn metric_lines(metric: &Value) -> String {
    if let Some(samples) = metric.get("samples").and_then(Value::as_array) {
        return samples.iter().map(metric_line).collect::<String>();
    }
    metric_line(metric)
}

fn metric_line(metric: &Value) -> String {
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
                .filter_map(|(key, value)| value.as_str().map(|raw| (key.as_str(), raw)))
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
    format!("{name}{{{labels}}} {value}\n")
}

fn trace_payload(trace: &Value) -> Value {
    let trace_id = hex_id(trace, "trace_id", 32);
    let now = now_nanos();
    let mut spans = vec![span_payload(trace, &trace_id, now)];
    for child in trace
        .get("child_spans")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        spans.push(span_payload(child, &trace_id, now));
    }
    json!({
        "resourceSpans": [{
            "resource": {"attributes": [
                attr("service.name", "ultragoal"),
                attr("run_id", str_field(trace, "run_id")),
                attr("candidate_digest", str_field(trace, "candidate_digest"))
            ]},
            "scopeSpans": [{
                "scope": {"name": "ultragoal"},
                "spans": spans
            }]
        }]
    })
}

fn span_payload(span: &Value, trace_id: &str, now: u128) -> Value {
    let mut payload = json!({
        "traceId": trace_id,
        "spanId": hex_id(span, "span_id", 16),
        "name": span
            .get("span_name")
            .and_then(Value::as_str)
            .unwrap_or_else(|| str_field(span, "operation")),
        "kind": 1,
        "startTimeUnixNano": now.to_string(),
        "endTimeUnixNano": (now + 1_000_000).to_string(),
        "attributes": [
            attr("run_id", str_field(span, "run_id")),
            attr("correlation_id", str_field(span, "correlation_id")),
            attr("operation", str_field(span, "operation")),
            attr("status", str_field(span, "status")),
            attr("law_id", str_field(span, "law_id")),
            attr("check_id", str_field(span, "check_id")),
            attr("claim_id", str_field(span, "claim_id")),
            attr("failure_class", str_field(span, "failure_class")),
            attr("why_failed", str_field(span, "why_failed")),
            attr("next_repair", str_field(span, "next_repair")),
            attr("claim_impact", str_field(span, "claim_impact")),
            attr("span_kind", str_field(span, "span_kind"))
        ]
    });
    if let Some(parent) = span.get("parent_span_id").and_then(Value::as_str) {
        if !parent.is_empty() {
            payload["parentSpanId"] = json!(hex_value(parent, 16));
        }
    }
    payload
}

fn attr(key: &str, value: &str) -> Value {
    json!({"key": key, "value": {"stringValue": value}})
}

fn str_field<'a>(value: &'a Value, field: &str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or("")
}

fn hex_id(value: &Value, field: &str, len: usize) -> String {
    let raw = str_field(value, field);
    hex_value(raw, len)
}

fn hex_value(raw: &str, len: usize) -> String {
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
    metric_lines(metric)
}

#[cfg(test)]
pub(crate) fn trace_payload_for_test(trace: &Value) -> Value {
    trace_payload(trace)
}
