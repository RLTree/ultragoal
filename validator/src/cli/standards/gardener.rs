use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const RECEIPT_DIR: &str = "validation_artifacts/standards-gardener/";
const RECEIPT: &str =
    "validation_artifacts/standards-gardener/current-standards-gardening-receipt.json";
const RUNTIME_ARTIFACT_PREFIX: &str = "validation_artifacts/";

pub(crate) fn rebind(root: &Path, receipt: &Path) -> Result<Value, String> {
    validate_receipt_path(root, receipt)?;
    let receipt_path = root.join(receipt);
    let mut value = crate::json_boundary::read_json(&receipt_path)?;
    let candidate = crate::package::inventory::package_digest(root)?;
    value["generated_at"] = json!(crate::audit::clock::now_iso());
    value["candidate_digest"] = json!(candidate);
    rebind_changed_artifacts(root, &mut value)?;
    validate_receipt(root, &value)?;
    crate::json_boundary::write_json(&receipt_path, &value)?;
    Ok(value)
}

fn validate_receipt_path(root: &Path, receipt: &Path) -> Result<(), String> {
    let text = receipt.to_string_lossy();
    if receipt.is_absolute() || !text.starts_with(RECEIPT_DIR) {
        return Err("standards-gardener receipt must be root-relative under validation_artifacts/standards-gardener".into());
    }
    if crate::package::inventory::package_path_error(root, &text).is_some() {
        return Err("standards-gardener receipt path escapes package root".into());
    }
    Ok(())
}

fn rebind_changed_artifacts(root: &Path, value: &mut Value) -> Result<(), String> {
    let rows = value
        .get_mut("changed_artifacts")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "standards-gardener receipt missing changed_artifacts".to_string())?;
    if rows.is_empty() {
        return Err("standards-gardener receipt changed_artifacts is empty".into());
    }
    for artifact in rows {
        let rel = artifact_path(artifact)?;
        let text = rel.to_string_lossy();
        if text == RECEIPT {
            return Err("standards-gardener receipt cannot track itself".into());
        }
        if text.starts_with(RUNTIME_ARTIFACT_PREFIX) {
            return Err(
                "standards-gardener receipt cannot track runtime validation_artifacts".into(),
            );
        }
        if crate::package::inventory::package_path_error(root, &text).is_some() {
            return Err(format!(
                "standards-gardener changed artifact path invalid: {text}"
            ));
        }
        artifact["digest"] = json!(crate::digest::file(&root.join(&rel))?);
    }
    Ok(())
}

fn artifact_path(artifact: &Value) -> Result<PathBuf, String> {
    artifact
        .get("path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| "standards-gardener changed_artifact missing path".to_string())
}

fn validate_receipt(root: &Path, value: &Value) -> Result<(), String> {
    let store = crate::schema_catalog::load(root);
    let mut failures = crate::audit::standards_gardening::receipt_failures(&store, value);
    failures.extend(crate::audit::standards_gardening::receipt_root_failures(
        root, value,
    ));
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "standards-gardener receipt did not validate: {}",
            failures.join(" | ")
        ))
    }
}
