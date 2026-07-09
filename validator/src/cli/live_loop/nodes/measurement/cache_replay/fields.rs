use serde_json::Value;
use std::time::Instant;

pub(in crate::cli::live_loop::nodes::measurement::cache_replay) fn node_rows(
    value: &Value,
) -> Vec<&Value> {
    let mut rows = Vec::new();
    if let Some(records) = value.get("records").and_then(Value::as_array) {
        rows.extend(records.iter());
    }
    if let Some(cache_records) = value.get("cache_records").and_then(Value::as_array) {
        rows.extend(cache_records.iter());
    }
    if let Some(nodes) = value.get("nodes").and_then(Value::as_array) {
        rows.extend(nodes.iter());
    }
    rows
}

pub(in crate::cli::live_loop::nodes::measurement::cache_replay) fn text<'a>(
    value: &'a Value,
    key: &str,
) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

pub(in crate::cli::live_loop::nodes::measurement::cache_replay) fn valid_digest(
    value: &str,
) -> Option<&str> {
    (value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit()))
    .then_some(value)
}

pub(in crate::cli::live_loop::nodes::measurement::cache_replay) fn json_string_array(
    row: &Value,
    field: &str,
) -> Option<Vec<String>> {
    row.get(field)?
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(ToString::to_string))
        .collect()
}

pub(in crate::cli::live_loop::nodes::measurement::cache_replay) fn elapsed_ms(
    started: Instant,
) -> u64 {
    u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}
