use serde_json::Value;
use std::path::{Path, PathBuf};

pub(super) fn coverage_manifest_path(root: &Path) -> PathBuf {
    let target = root.join(".harness/coverage-manifest.json");
    if target.is_file() {
        target
    } else {
        root.join("templates/.harness/coverage-manifest.json")
    }
}

pub(super) fn coverage_command_path(root: &Path) -> PathBuf {
    let target = root.join(".harness/coverage-command");
    if target.is_file() {
        target
    } else {
        root.join("templates/.harness/coverage-command")
    }
}

pub(super) fn resolve_receipt(root: &Path, receipt: &Path) -> Result<PathBuf, String> {
    crate::output_path::claim_artifact_path(root, receipt, "coverage receipt")
}

pub(super) fn string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

pub(super) fn array_strings(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

pub(super) fn array_contains(value: &Value, key: &str, needle: &str) -> bool {
    array_strings(value, key).iter().any(|item| item == needle)
}

pub(super) fn sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}
