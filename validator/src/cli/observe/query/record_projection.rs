use serde_json::{Value, json};

pub(crate) fn observed_telemetry_record(rows: &[Value]) -> Value {
    rows.iter()
        .filter_map(|row| row.get("body").and_then(Value::as_str))
        .find_map(record_in_body)
        .unwrap_or_else(|| json!({}))
}

fn record_in_body(body: &str) -> Option<Value> {
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        return first_target_record(&value);
    }
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find_map(|value| first_target_record(&value))
}

fn first_target_record(value: &Value) -> Option<Value> {
    match value {
        Value::Object(map) => {
            if let Some(record) = trace_span_record(value) {
                return Some(record);
            }
            if is_target_record(value) {
                return Some(project_record(value));
            }
            map.values().find_map(first_target_record)
        }
        Value::Array(items) => items.iter().find_map(first_target_record),
        Value::String(raw) if starts_with_json(raw) => serde_json::from_str::<Value>(raw)
            .ok()
            .and_then(|nested| first_target_record(&nested)),
        _ => None,
    }
}

fn is_target_record(value: &Value) -> bool {
    let Some(operation) = semantic_text_field(value, "operation") else {
        return false;
    };
    !is_observe_query_or_explain(operation)
        && semantic_text_field(value, "candidate_digest").is_some()
}

fn project_record(value: &Value) -> Value {
    project_record_with_candidate(
        value,
        semantic_text_field(value, "candidate_digest").unwrap_or("none"),
    )
}

fn project_record_with_candidate(value: &Value, candidate_digest: &str) -> Value {
    json!({
        "operation": semantic_text_field(value, "operation").unwrap_or("none"),
        "status": semantic_text_field(value, "status").unwrap_or("none"),
        "run_id": semantic_text_field(value, "run_id").unwrap_or("none"),
        "correlation_id": semantic_text_field(value, "correlation_id").unwrap_or("none"),
        "trace_id": semantic_text_field(value, "trace_id").unwrap_or("none"),
        "span_id": semantic_text_field(value, "span_id").unwrap_or("none"),
        "candidate_digest": candidate_digest,
        "failure_class": semantic_text_field(value, "failure_class").unwrap_or("none"),
        "why_failed": semantic_text_field(value, "why_failed").unwrap_or("none"),
        "where_failed": semantic_text_field(value, "where_failed").unwrap_or("none"),
        "next_repair": semantic_text_field(value, "next_repair").unwrap_or("none"),
        "claim_impact": semantic_text_field(value, "claim_impact").unwrap_or("none"),
        "law_id": semantic_text_field(value, "law_id").unwrap_or("none"),
        "check_id": semantic_text_field(value, "check_id").unwrap_or("none"),
        "claim_id": semantic_text_field(value, "claim_id").unwrap_or("none")
    })
}

fn trace_span_record(value: &Value) -> Option<Value> {
    let processes = value.get("processes").and_then(Value::as_object)?;
    let spans = value.get("spans").and_then(Value::as_array)?;
    spans.iter().find_map(|span| {
        let operation = semantic_text_field(span, "operation")?;
        if is_observe_query_or_explain(operation) {
            return None;
        }
        let process_id = span.get("processID").and_then(Value::as_str)?;
        let process = processes.get(process_id).unwrap_or(&Value::Null);
        let candidate = semantic_text_field(span, "candidate_digest")
            .or_else(|| semantic_text_field(process, "candidate_digest"))
            .unwrap_or("none");
        Some(project_record_with_candidate(span, candidate))
    })
}

fn semantic_text_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .or_else(|| tag_array_text_field(value, field))
}

fn tag_array_text_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value
        .get("tags")
        .and_then(Value::as_array)?
        .iter()
        .find_map(|tag| {
            (tag.get("key").and_then(Value::as_str) == Some(field))
                .then(|| tag.get("value").and_then(Value::as_str))
                .flatten()
        })
}

fn is_observe_query_or_explain(operation: &str) -> bool {
    operation.starts_with("observe.explain-")
        || matches!(
            operation,
            "observe.logs.query" | "observe.metrics.query" | "observe.traces.query"
        )
}

fn starts_with_json(raw: &str) -> bool {
    raw.trim_start()
        .chars()
        .next()
        .is_some_and(|ch| matches!(ch, '{' | '['))
}
