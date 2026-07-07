use serde_json::Value;
use std::time::Instant;

pub(super) fn node_rows(value: &Value) -> Vec<&Value> {
    let mut rows = Vec::new();
    if let Some(cache_records) = value.get("cache_records").and_then(Value::as_array) {
        rows.extend(cache_records.iter());
    }
    if let Some(nodes) = value.get("nodes").and_then(Value::as_array) {
        rows.extend(nodes.iter());
    }
    rows
}

pub(super) fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

pub(super) fn valid_digest(value: &str) -> Option<&str> {
    (value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit()))
    .then_some(value)
}

pub(super) fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}
