use serde_json::Value;

pub(super) fn command_receipt_identity_missing(value: &Value) -> bool {
    required_fields()
        .iter()
        .any(|field| value.get(field).and_then(Value::as_str).is_none())
}

fn required_fields() -> &'static [&'static str] {
    &[
        "candidate_digest",
        "run_id",
        "correlation_id",
        "operation",
        "receipt_path",
    ]
}
