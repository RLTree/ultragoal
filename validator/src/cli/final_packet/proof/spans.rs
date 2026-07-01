use serde_json::{Value, json};

pub(super) fn attach(proof: &Value, observability: &mut Value) {
    let event = observability["event"].clone();
    let Some(children) = observability
        .pointer_mut("/trace/child_spans")
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    let root_span = event
        .get("span_id")
        .and_then(Value::as_str)
        .unwrap_or("span-root");
    for row in dereferenced_receipts(proof) {
        children.push(child_span(&event, root_span, &row));
    }
    observability["trace_bundle_digest"] =
        json!(crate::digest::canonical_json(&observability["trace"]));
}

struct ReceiptSpan {
    label: String,
    path: String,
    status: String,
    digest: String,
}

fn dereferenced_receipts(proof: &Value) -> Vec<ReceiptSpan> {
    let mut rows = Vec::new();
    for key in [
        "cli_performance",
        "registry_exposure",
        "source_audit",
        "coverage",
    ] {
        if let Some(row) = proof.get(key) {
            rows.push(receipt_span(key, row));
        }
    }
    for (index, row) in proof
        .get("package_receipts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        rows.push(receipt_span(&format!("package_receipt_{index}"), row));
    }
    rows
}

fn receipt_span(label: &str, row: &Value) -> ReceiptSpan {
    ReceiptSpan {
        label: label.to_string(),
        path: text(row, "path").to_string(),
        status: text(row, "status").to_string(),
        digest: text(row, "digest").to_string(),
    }
}

fn child_span(event: &Value, root_span: &str, row: &ReceiptSpan) -> Value {
    let operation = event
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or("final-packet.prove");
    let mut span = event.clone();
    span["schema"] = json!("harness-ultragoal.observability-trace.v1");
    span["span_id"] = json!(child_span_id(root_span, &row.label));
    span["parent_span_id"] = json!(root_span);
    span["span_kind"] = json!("receipt_deref");
    span["span_name"] = json!(format!("{operation}.deref.{}", row.label));
    span["receipt_path"] = json!(row.path);
    span["dereferenced_receipt_label"] = json!(row.label);
    span["dereferenced_receipt_status"] = json!(row.status);
    span["dereferenced_receipt_digest"] = json!(row.digest);
    span["child_spans"] = Value::Array(Vec::new());
    span
}

fn child_span_id(root_span: &str, label: &str) -> String {
    let digest = crate::digest::bytes(format!("{root_span}:final-packet:{label}").as_bytes());
    format!("span-{}", &digest["sha256:".len()..34])
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}
