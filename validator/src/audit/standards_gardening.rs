use crate::{digest, json_boundary, schema_catalog};
use schema_catalog::SchemaStore;
use serde_json::Value;
use std::path::Path;

const ROW_ID: &str = "standards-gardener-promotion";
const RECEIPT_KEY: &str = "standards_gardening_receipt_path";

pub fn failures(root: &Path, store: &SchemaStore) -> Vec<String> {
    let matrix = match json_boundary::read_json(&root.join("docs/source-obligation-matrix.json")) {
        Ok(value) => value,
        Err(err) => return vec![format!("standards_gardener_matrix_unreadable: {err}")],
    };
    let mut out = value_failures(&matrix);
    out.extend(root_value_failures(root, store, &matrix));
    out
}

pub fn value_failures(value: &Value) -> Vec<String> {
    let Some(row) = row(value) else {
        return Vec::new();
    };
    if row.get(RECEIPT_KEY).is_none() {
        return vec!["standards_gardener_receipt_missing".to_string()];
    }
    Vec::new()
}

fn root_value_failures(root: &Path, store: &SchemaStore, value: &Value) -> Vec<String> {
    let Some(row) = row(value) else {
        return Vec::new();
    };
    let Some(path) = row.get(RECEIPT_KEY).and_then(Value::as_str) else {
        return Vec::new();
    };
    if crate::package::inventory::package_path_error(root, path).is_some() {
        return vec!["standards_gardener_receipt_artifact_invalid".to_string()];
    }
    let receipt = match json_boundary::read_json(&root.join(path)) {
        Ok(value) => value,
        Err(_) => return vec!["standards_gardener_receipt_artifact_invalid".to_string()],
    };
    let mut out = receipt_failures(store, &receipt);
    out.extend(receipt_root_failures(root, &receipt));
    out
}

pub fn receipt_failures(store: &SchemaStore, receipt: &Value) -> Vec<String> {
    let schema_errors =
        schema_catalog::schema_errors(store, "standards-gardening-receipt.schema.json", receipt);
    if !schema_errors.is_empty() {
        return vec!["standards_gardener_receipt_schema_invalid".to_string()];
    }
    let severity = receipt
        .pointer("/trigger_signal/severity")
        .and_then(Value::as_str)
        .unwrap_or("");
    let action = receipt
        .pointer("/decision/action")
        .and_then(Value::as_str)
        .unwrap_or("");
    if severity == "low" || action == "backlog" {
        return vec!["standards_gardener_receipt_semantic_invalid".to_string()];
    }
    if action == "hook"
        && receipt
            .pointer("/safeguards/hook_justification")
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty()
    {
        return vec!["standards_gardener_receipt_semantic_invalid".to_string()];
    }
    Vec::new()
}

fn changed_artifact_failures(root: &Path, receipt: &Value) -> Vec<String> {
    receipt
        .get("changed_artifacts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|artifact| artifact_mismatch(root, artifact))
        .collect()
}

pub fn receipt_root_failures(root: &Path, receipt: &Value) -> Vec<String> {
    let mut out = changed_artifact_failures(root, receipt);
    out.extend(changed_artifact_time_failures(root, receipt));
    out
}

fn artifact_mismatch(root: &Path, artifact: &Value) -> Option<String> {
    let path = artifact.get("path").and_then(Value::as_str).unwrap_or("");
    let got = artifact.get("digest").and_then(Value::as_str).unwrap_or("");
    let resolved = crate::package::inventory::resolve(root, path).ok()?;
    if !resolved.is_file() {
        return Some("standards_gardener_changed_artifact_missing".to_string());
    }
    match digest::file(&resolved) {
        Ok(actual) if actual == got => None,
        _ => Some("standards_gardener_changed_artifact_digest_mismatch".to_string()),
    }
}

fn changed_artifact_time_failures(root: &Path, receipt: &Value) -> Vec<String> {
    let generated_at = receipt
        .get("generated_at")
        .and_then(Value::as_str)
        .unwrap_or("");
    let Some(receipt_time) = crate::audit::clock::parse_iso_seconds(generated_at) else {
        return vec!["standards_gardener_receipt_semantic_invalid".to_string()];
    };
    receipt
        .get("changed_artifacts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|artifact| artifact_time_after_receipt(root, artifact, receipt_time))
        .collect()
}

fn artifact_time_after_receipt(root: &Path, artifact: &Value, receipt_time: i64) -> Option<String> {
    let path = artifact.get("path").and_then(Value::as_str).unwrap_or("");
    let resolved = crate::package::inventory::resolve(root, path).ok()?;
    let value = json_boundary::read_json(&resolved).ok()?;
    let artifact_time = ["captured_at", "generated_at", "validated_at"]
        .iter()
        .filter_map(|key| value.get(*key).and_then(Value::as_str))
        .filter_map(crate::audit::clock::parse_iso_seconds)
        .max()?;
    if artifact_time > receipt_time {
        Some("standards_gardener_changed_artifact_after_receipt".to_string())
    } else {
        None
    }
}

fn row(value: &Value) -> Option<&Value> {
    value
        .get("obligations")?
        .as_array()?
        .iter()
        .find(|row| row.get("id").and_then(Value::as_str) == Some(ROW_ID))
}
