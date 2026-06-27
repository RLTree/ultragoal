use serde_json::Value;

pub fn required(value: &Value, keys: &[&str], base: &str, errors: &mut Vec<String>) {
    for key in keys {
        if value.get(*key).is_none() {
            errors.push(path(base, key, "is required"));
        }
    }
}

pub fn nonempty(value: &Value, key: &str, label: &str, errors: &mut Vec<String>) {
    if value
        .get(key)
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        errors.push(format!("{label} must be a non-empty string"));
    }
}

pub fn artifact(value: &Value, label: &str, errors: &mut Vec<String>) {
    nonempty(value, "path", &format!("{label}.path"), errors);
    let digest = value.get("digest").and_then(Value::as_str).unwrap_or("");
    if !is_sha(digest) {
        errors.push(format!("{label}.digest must be sha256"));
    }
}

pub fn array_value<'a>(
    value: &'a Value,
    label: &str,
    min: usize,
    errors: &mut Vec<String>,
) -> Vec<&'a Value> {
    let Some(rows) = value.as_array() else {
        errors.push(format!("{label} must be an array"));
        return Vec::new();
    };
    if rows.len() < min {
        errors.push(format!("{label} must contain at least {min} items"));
    }
    rows.iter().collect()
}

pub fn string_array(value: &Value, label: &str, errors: &mut Vec<String>) {
    string_array_min(value, label, 0, errors);
}

pub fn string_array_min(value: &Value, label: &str, min: usize, errors: &mut Vec<String>) {
    let Some(rows) = value.as_array() else {
        errors.push(format!("{label} must be an array"));
        return;
    };
    if rows.len() < min
        || rows
            .iter()
            .any(|row| row.as_str().is_none_or(str::is_empty))
    {
        errors.push(format!("{label} must contain non-empty strings"));
    }
}

pub fn string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn path(base: &str, key: &str, suffix: &str) -> String {
    if base.is_empty() {
        format!("{key} {suffix}")
    } else {
        format!("{base}.{key} {suffix}")
    }
}

fn is_sha(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].chars().all(|ch| ch.is_ascii_hexdigit())
}
