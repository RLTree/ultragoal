use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn red_fixture_ids(root: &Path) -> BTreeSet<String> {
    crate::json_boundary::read_json(&root.join("templates/RED_FIXTURES.json"))
        .ok()
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.get("id").and_then(Value::as_str).map(str::to_string))
        .collect()
}

pub(super) fn standards_row_exists(root: &Path, id: &str) -> bool {
    json_rows(root, "templates/agent-standards/enforcement.json", "rows")
        .iter()
        .any(|row| row.get("id").and_then(Value::as_str) == Some(id))
}

pub(super) fn source_obligation_exists(root: &Path, id: &str) -> bool {
    json_rows(root, "docs/source-obligation-matrix.json", "obligations")
        .iter()
        .any(|row| row.get("id").and_then(Value::as_str) == Some(id))
}

pub(super) fn trace_entry_exists(root: &Path, id: &str) -> bool {
    json_rows(root, "docs/foundational-law-traceability.json", "entries")
        .iter()
        .any(|row| row.get("obligation_id").and_then(Value::as_str) == Some(id))
}

fn json_rows(root: &Path, path: &str, key: &str) -> Vec<Value> {
    crate::json_boundary::read_json(&root.join(path))
        .ok()
        .and_then(|value| value.get(key).and_then(Value::as_array).cloned())
        .unwrap_or_default()
}
