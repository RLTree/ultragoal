use crate::audit::contract::CHECK_IDS;
use crate::digest;
use crate::schema_catalog::SchemaStore;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

pub fn check_ids(store: &SchemaStore) -> Vec<String> {
    store
        .schemas
        .get("schema-authority-primitives.schema.json")
        .and_then(|schema| schema.pointer("/$defs/requiredValidatorCheckId/enum"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_else(|| CHECK_IDS.iter().map(|id| id.to_string()).collect())
}

pub fn validator_artifacts(root: &Path) -> Result<Vec<Value>, String> {
    let mut paths = vec![
        "Cargo.toml".to_string(),
        "Cargo.lock".to_string(),
        "validator/Cargo.toml".to_string(),
    ];
    for entry in walkdir::WalkDir::new(root.join("validator/src"))
        .into_iter()
        .flatten()
    {
        if entry.file_type().is_file()
            && entry.path().extension().and_then(|ext| ext.to_str()) == Some("rs")
        {
            let rel = validator_artifact_rel(root, entry.path())?;
            paths.push(rel);
        }
    }
    paths.sort();
    paths.dedup();
    paths
        .into_iter()
        .map(|rel| artifact_ref(root, &rel))
        .collect()
}

pub fn digest_map(artifacts: &[Value]) -> BTreeMap<String, String> {
    artifacts
        .iter()
        .filter_map(|row| {
            Some((
                row.get("path")?.as_str()?.to_string(),
                row.get("digest")?.as_str()?.to_string(),
            ))
        })
        .collect()
}

pub fn input_refs(root: &Path) -> Result<Vec<Value>, String> {
    [
        "plugin-manifest-draft.json",
        "schemas/schema-catalog.json",
        "templates/RED_FIXTURES.json",
        "fixtures/valid/minimal-goal-run.json",
    ]
    .iter()
    .map(|rel| artifact_ref(root, rel))
    .collect()
}

pub fn safe_red_ids(root: &Path) -> Vec<String> {
    crate::json_boundary::read_json(&root.join("templates/RED_FIXTURES.json"))
        .ok()
        .and_then(|value| value.as_array().cloned())
        .map(|rows| {
            rows.into_iter()
                .filter_map(|row| row.get("id").and_then(Value::as_str).map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_else(|| vec!["red-catalog-unavailable".to_string()])
}

pub(crate) fn validator_artifact_rel(root: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(root)
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .map_err(|err| format!("validator artifact root strip failed: {err}"))
}

pub(crate) fn artifact_ref(root: &Path, rel: &str) -> Result<Value, String> {
    let path = crate::package::inventory::resolve(root, rel)?;
    let digest = artifact_digest_or_zero(&path)?;
    Ok(json!({"path": rel, "digest": digest}))
}

pub(crate) fn artifact_digest_or_zero(path: &Path) -> Result<String, String> {
    if path.is_file() {
        digest::file(path)
    } else {
        Ok(digest::ZERO.to_string())
    }
}
