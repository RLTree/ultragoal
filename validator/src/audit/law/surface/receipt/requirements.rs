use serde_json::Value;
use std::path::Path;

pub(super) fn require_schema(value: &Value, expected: &str, error: &str, out: &mut Vec<String>) {
    if value.get("schema").and_then(Value::as_str) != Some(expected) {
        out.push(error.to_string());
    }
}

pub(super) fn require_nonempty(
    value: &Value,
    key: &str,
    error: &str,
    out: &mut Vec<String>,
) -> Option<String> {
    let found = value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .map(str::to_string);
    if found.is_none() {
        out.push(error.to_string());
    }
    found
}

pub(super) fn array_contains(value: &Value, key: &str, expected: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|row| row.as_str() == Some(expected))
}

pub(super) fn artifact_digest_failures(
    root: &Path,
    value: &Value,
    key: &str,
    error: &str,
    out: &mut Vec<String>,
) {
    for artifact in value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(path) = artifact.get("path").and_then(Value::as_str) else {
            out.push(error.to_string());
            continue;
        };
        let Some(expected) = artifact.get("digest").and_then(Value::as_str) else {
            out.push(error.to_string());
            continue;
        };
        match crate::digest::file(&root.join(path)) {
            Ok(actual) if expected == actual => {}
            _ => out.push(error.to_string()),
        }
    }
}
