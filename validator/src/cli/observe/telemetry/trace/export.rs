use serde_json::{Value, json};
use std::time::{SystemTime, UNIX_EPOCH};

pub(in crate::cli::observe::telemetry) fn payload(trace: &Value) -> Value {
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
                attr("correlation_id", str_field(trace, "correlation_id")),
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
    hex_value(str_field(value, field), len)
}

fn hex_value(raw: &str, len: usize) -> String {
    crate::digest::bytes(raw.as_bytes())
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
