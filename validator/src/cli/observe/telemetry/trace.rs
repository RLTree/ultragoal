use serde_json::{Value, json};

pub(crate) fn from_event(event: &Value) -> Value {
    let mut span = event.clone();
    span["schema"] = json!("harness-ultragoal.observability-trace.v1");
    span["exporter"] = if event["exporter"].as_str() == Some("receipt") {
        json!("receipt")
    } else {
        json!("victoriatraces")
    };
    span["span_kind"] = json!("root");
    span["span_name"] = event["operation"].clone();
    span["child_spans"] = json!(child_spans(event));
    span
}

fn child_spans(event: &Value) -> Vec<Value> {
    [
        ("validator_check", "validator-check"),
        ("receipt_deref", "receipt-binding"),
        ("claim_ceiling", "claim-ceiling"),
        ("exporter_call", "telemetry-export"),
    ]
    .into_iter()
    .map(|(kind, suffix)| child_span(event, kind, suffix))
    .collect()
}

fn child_span(event: &Value, kind: &str, suffix: &str) -> Value {
    let mut span = event.clone();
    let operation = event
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let root_span = event
        .get("span_id")
        .and_then(Value::as_str)
        .unwrap_or("span-root");
    span["schema"] = json!("harness-ultragoal.observability-trace.v1");
    span["span_id"] = json!(child_span_id(root_span, suffix));
    span["parent_span_id"] = json!(root_span);
    span["span_kind"] = json!(kind);
    span["span_name"] = json!(format!("{operation}.{suffix}"));
    span["child_spans"] = Value::Array(Vec::new());
    span
}

fn child_span_id(root_span: &str, suffix: &str) -> String {
    let digest = crate::digest::bytes(format!("{root_span}:{suffix}").as_bytes());
    format!("span-{}", &digest["sha256:".len()..34])
}
