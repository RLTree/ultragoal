use crate::digest;
use crate::json_boundary;
use serde_json::{Value, json};
use std::path::Path;

pub const REVIEW_EXCLUDED_PREFIXES: &[&str] =
    crate::package::inventory::PACKAGE_DIGEST_EXCLUDED_PREFIXES;
pub const REVIEW_EXCLUDED_PATHS: &[&str] = crate::package::inventory::PACKAGE_DIGEST_EXCLUDED_PATHS;

pub fn build_review_target_receipt(root: &Path) -> Result<Value, String> {
    let manifest = json_boundary::read_json(&root.join("plugin-manifest-draft.json"))?;
    let (included, excluded) = review_target_paths(&manifest);
    review_target_closed(root, &included)?;
    let path_list_payload = included.join("\n").into_bytes();
    let mut receipt = json!({
        "schema": "harness-ultragoal.review-target-receipt.v1",
        "status": "pass",
        "generated_at": crate::audit::clock::now_iso(),
        "root": ".",
        "package_digest": crate::package::inventory::package_digest(root)?,
        "review_target_digest": hash_entries(root, &included)?,
        "digest_algorithm": "sha256 over sorted review target paths as relpath NUL content-bytes NUL",
        "manifest_normalization": {
            "plugin_manifest": "hashes plugin-manifest-draft.json with detached review/proof paths removed",
            "excluded_prefixes": REVIEW_EXCLUDED_PREFIXES,
            "excluded_paths": REVIEW_EXCLUDED_PATHS
        },
        "included_path_count": included.len(),
        "included_path_list_digest": digest::bytes(&path_list_payload),
        "excluded_path_count": excluded.len(),
        "excluded_paths": excluded
    });
    let validator_receipt =
        root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
    if validator_receipt.is_file() {
        receipt["validator_receipt"] = json!({
            "path": "validation_artifacts/ultragoal-audit/validator-receipt.json",
            "digest": digest::file(&validator_receipt)?
        });
    }
    Ok(receipt)
}

pub fn review_excluded(rel: &str) -> bool {
    REVIEW_EXCLUDED_PATHS.contains(&rel)
        || REVIEW_EXCLUDED_PREFIXES
            .iter()
            .any(|prefix| rel.starts_with(prefix))
}

pub fn normalized_manifest_for_review(manifest: &Value) -> Value {
    let mut normalized = manifest.clone();
    for key in [
        "schemas",
        "fixtures",
        "authorable_templates",
        "generated_examples",
        "resources",
    ] {
        if let Some(items) = normalized.get_mut(key).and_then(Value::as_array_mut) {
            items.retain(|item| item.as_str().is_none_or(|rel| !review_excluded(rel)));
        }
    }
    for key in ["skills", "agents"] {
        if let Some(items) = normalized.get_mut(key).and_then(Value::as_array_mut) {
            items.retain(|item| {
                item.get("path")
                    .and_then(Value::as_str)
                    .is_none_or(|rel| !review_excluded(rel))
            });
        }
    }
    if normalized
        .get("schema_catalog")
        .and_then(Value::as_str)
        .is_some_and(review_excluded)
    {
        normalized
            .as_object_mut()
            .map(|obj| obj.remove("schema_catalog"));
    }
    normalized
}

pub fn review_payload(root: &Path, rel: &str) -> Result<Vec<u8>, String> {
    if rel == "plugin-manifest-draft.json" {
        let manifest = json_boundary::read_json(&crate::package::inventory::resolve(root, rel)?)?;
        return Ok(normalized_manifest_for_review(&manifest)
            .to_string()
            .into_bytes());
    }
    let path = crate::package::inventory::resolve(root, rel)?;
    if path.is_file() {
        let bytes = digest::read_file_bytes(&path)
            .map_err(|err| format!("{}: review payload read failed: {err}", path.display()))
            .and_then(|bytes| crate::package::inventory::stable_package_payload(rel, &bytes))?;
        Ok(bytes)
    } else {
        Err(format!(
            "{}: review target manifest path is missing",
            path.display()
        ))
    }
}

fn review_target_paths(manifest: &Value) -> (Vec<String>, Vec<String>) {
    let mut included = Vec::new();
    let mut excluded = Vec::new();
    let mut paths = crate::package::inventory::inventory_paths(manifest);
    paths.sort();
    paths.dedup();
    for rel in paths {
        if review_excluded(&rel) {
            excluded.push(rel);
        } else {
            included.push(rel);
        }
    }
    (included, excluded)
}

fn review_target_closed(root: &Path, included: &[String]) -> Result<(), String> {
    let invalid = included
        .iter()
        .filter_map(|rel| {
            crate::package::inventory::package_path_error(root, rel)
                .map(|err| format!("{rel}: {err}"))
        })
        .collect::<Vec<_>>();
    let missing = included
        .iter()
        .filter(|rel| {
            crate::package::inventory::resolve(root, rel)
                .map(|p| !p.is_file())
                .unwrap_or(true)
        })
        .cloned()
        .collect::<Vec<_>>();
    if invalid.is_empty() && missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "review target is not closed: invalid={invalid:?} missing={missing:?}"
        ))
    }
}

fn hash_entries(root: &Path, paths: &[String]) -> Result<String, String> {
    let mut payload = Vec::new();
    for rel in paths {
        payload.extend_from_slice(rel.as_bytes());
        payload.push(0);
        payload.extend_from_slice(&review_payload(root, rel)?);
        payload.push(0);
    }
    Ok(digest::bytes(&payload))
}
pub(crate) mod artifact;
pub(crate) mod inventory;
pub(crate) mod resource;
