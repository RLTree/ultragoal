use serde_json::Value;

pub(super) fn node_rows(value: &Value) -> Vec<&Value> {
    value
        .get("nodes")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().collect())
        .unwrap_or_default()
}

pub(super) fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

pub(super) fn nonempty_text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    let text = text(value, key)?;
    (!text.is_empty()).then_some(text)
}

pub(super) fn positive(value: &Value, key: &str) -> Option<u64> {
    let number = value.get(key).and_then(Value::as_u64)?;
    (number > 0).then_some(number)
}

pub(super) fn valid_digest(value: &str) -> Option<&str> {
    (value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit()))
    .then_some(value)
}

pub(super) fn has_nonempty_string_array(value: &Value, key: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| !items.is_empty() && items.iter().all(|item| item.as_str().is_some()))
        .unwrap_or(false)
}
