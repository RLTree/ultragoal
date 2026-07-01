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
    span
}
