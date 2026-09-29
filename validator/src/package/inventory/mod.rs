use crate::digest;
use serde_json::Value;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

pub(crate) mod anchored;
pub(crate) mod closure;
pub(crate) mod draft_manifest;
pub(crate) mod generated_disposition;
pub(crate) mod payload;
pub(crate) mod snapshot;

pub use closure::inventory_closure_failures;
pub(crate) use draft_manifest::DraftPackageManifest;

pub const PACKAGE_DIGEST_EXCLUDED_PREFIXES: &[&str] = &["validation_artifacts/"];
pub const PACKAGE_DIGEST_EXCLUDED_PATHS: &[&str] = &[];
const PACKAGE_CONTRACT_COMPATIBILITY_PREFIX: &str = "docs/package-contract-compatibility-";
const BUILDER_CONTRACT_MODULE_DIR: &str = "docs/ultragoal-contract-2026-07/";
const MANIFEST_PATH: &str = "plugin-manifest-draft.json";

pub fn inventory_paths(manifest: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for key in ["skills", "agents"] {
        if let Some(rows) = manifest.get(key).and_then(Value::as_array) {
            for row in rows {
                if let Some(path) = row.get("path").and_then(Value::as_str) {
                    out.push(path.to_string());
                }
            }
        }
    }
    if let Some(path) = manifest.get("schema_catalog").and_then(Value::as_str) {
        out.push(path.to_string());
    }
    for key in [
        "schemas",
        "fixtures",
        "authorable_templates",
        "generated_examples",
        "resources",
    ] {
        if let Some(rows) = manifest.get(key).and_then(Value::as_array) {
            for row in rows {
                if let Some(path) = row.as_str() {
                    out.push(path.to_string());
                }
            }
        }
    }
    out
}

pub fn package_path_error(root: &Path, rel: &str) -> Option<String> {
    if let Some(error) = package_path_syntax_error(rel) {
        return Some(error);
    }
    let root_abs = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    if symlink_component_error(&root_abs, rel).is_some() {
        return Some(format!("package path uses symlink: {rel}"));
    }
    let full = root_abs.join(rel);
    let full = full.canonicalize().unwrap_or(full);
    canonical_escape_error(&root_abs, &full, rel)
}

pub(crate) fn builder_contract_resource_path(rel: &str) -> bool {
    (rel.starts_with(PACKAGE_CONTRACT_COMPATIBILITY_PREFIX) && rel.ends_with(".md"))
        || rel.starts_with(BUILDER_CONTRACT_MODULE_DIR)
}

pub fn resolve(root: &Path, rel: &str) -> Result<PathBuf, String> {
    if let Some(error) = package_path_error(root, rel) {
        return Err(error);
    }
    Ok(root.join(Path::new(rel)))
}

fn symlink_component_error(root: &Path, rel: &str) -> Option<()> {
    let mut cursor = root.to_path_buf();
    for name in Path::new(rel).iter() {
        cursor.push(name);
        let Ok(meta) = std::fs::symlink_metadata(&cursor) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            return Some(());
        }
    }
    None
}

fn canonical_escape_error(root: &Path, full: &Path, rel: &str) -> Option<String> {
    if full.strip_prefix(root).is_err() {
        return Some(format!("package path escapes package root: {rel}"));
    }
    None
}

pub fn package_digest_excluded(rel: &str) -> bool {
    PACKAGE_DIGEST_EXCLUDED_PATHS.contains(&rel)
        || PACKAGE_DIGEST_EXCLUDED_PREFIXES
            .iter()
            .any(|prefix| rel.starts_with(prefix))
}

pub fn package_digest(root: &Path) -> Result<String, String> {
    let mut session = anchored::Session::open(root)
        .map_err(|error| format!("{MANIFEST_PATH}: package manifest unavailable: {error}"))?;
    let manifest_bytes = session
        .read(MANIFEST_PATH, anchored::MAX_MANIFEST_BYTES)
        .map_err(|error| format!("{MANIFEST_PATH}: package manifest unavailable: {error}"))?;
    let manifest = anchored::parse_unique_json(&manifest_bytes)?;
    let mut paths = inventory_paths(&manifest)
        .into_iter()
        .filter(|rel| !package_digest_excluded(rel))
        .collect::<Vec<_>>();
    paths.sort();
    if paths.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("package manifest contains a duplicate inventory path".to_string());
    }
    let dispositions = generated_disposition::Catalog::for_manifest_in(&mut session, &paths)?;
    let mut rows = Vec::new();
    if let Some(catalog) = dispositions.as_ref() {
        rows.push((
            generated_disposition::REGISTRY_PATH.to_string(),
            Arc::<[u8]>::from(catalog.registry_bytes()),
        ));
    }
    for rel in paths {
        if generated_disposition::generated_path(&rel) {
            let catalog = dispositions
                .as_ref()
                .expect("generated path requires a disposition catalog");
            match catalog.classify_in(&mut session, &rel)? {
                generated_disposition::Classification::AdoptedSchemaContract => {}
                generated_disposition::Classification::RetainedContext { .. } => continue,
            }
        }
        if rel == generated_disposition::REGISTRY_PATH && dispositions.is_some() {
            continue;
        }
        let bytes = Arc::<[u8]>::from(package_resource_bytes(&mut session, &rel)?);
        rows.push((rel, bytes));
    }
    session.finish()?;
    rows.sort_by(|left, right| left.0.cmp(&right.0));
    let mut payload = Vec::new();
    for (rel, bytes) in rows {
        payload.extend_from_slice(rel.as_bytes());
        payload.push(0);
        payload.extend_from_slice(&payload::stable_package_payload(&rel, bytes.as_ref())?);
        payload.push(0);
    }
    Ok(digest::bytes(&payload))
}

fn package_resource_bytes(session: &mut anchored::Session, rel: &str) -> Result<Vec<u8>, String> {
    if let Some(error) = package_path_syntax_error(rel) {
        return Err(format!("package digest path invalid: {error}"));
    }
    session
        .read(rel, anchored::MAX_RESOURCE_BYTES)
        .map_err(|error| format!("package digest resource unavailable: {error}"))
}

pub(crate) fn package_path_syntax_error(rel: &str) -> Option<String> {
    if rel.is_empty() {
        return Some("package path is not a non-empty string".to_string());
    }
    if builder_contract_resource_path(rel) {
        return Some(format!(
            "builder contract resource is not a package resource: {rel}"
        ));
    }
    if Path::new(rel).is_absolute() {
        return Some(format!("package path is absolute: {rel}"));
    }
    if Path::new(rel)
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Some(format!("package path escapes package root: {rel}"));
    }
    None
}

#[cfg(test)]
mod tests;
