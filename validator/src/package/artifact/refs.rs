use crate::digest;
use serde_json::Value;
use std::fs::Metadata;
use std::path::{Component, Path};

pub fn validate_path_digest(root: &Path, path: &str, got: &str, label: &str) -> Result<(), String> {
    if path.is_empty() || got.is_empty() || got == digest::ZERO {
        return Err(format!("{label} lacks non-zero artifact digest"));
    }
    if placeholder_path(path) {
        return Err(format!("{label} uses placeholder artifact path: {path}"));
    }
    let rel = Path::new(path);
    if rel.is_absolute() {
        return Err(format!("{label} uses absolute artifact path"));
    }
    if rel
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!("{label} escapes package root"));
    }
    reject_symlink_component(root, rel, label)?;
    let resolved = crate::package::inventory::resolve(root, path)?;
    let canonical_root = package_root_result(root.canonicalize())?;
    let canonical = package_artifact_canonical_result(label, path, resolved.canonicalize())?;
    ensure_inside_package(&canonical_root, &canonical, label)?;
    let meta =
        package_artifact_metadata_result(label, path, std::fs::symlink_metadata(&canonical))?;
    if meta.file_type().is_symlink() || !meta.file_type().is_file() {
        return Err(format!("{label} artifact is not a regular file: {path}"));
    }
    match package_artifact_digest_result(digest::file(&canonical)) {
        Ok(actual) if actual == got => Ok(()),
        Ok(_) => Err(format!("{label} digest mismatch: {path}")),
        Err(_) => Err(format!("{label} artifact unreadable: {path}")),
    }
}

pub fn validate_object(root: &Path, item: &Value, label: &str) -> Result<(), String> {
    let path = item.get("path").and_then(Value::as_str).unwrap_or("");
    let got = item.get("digest").and_then(Value::as_str).unwrap_or("");
    validate_path_digest(root, path, got, label)
}

pub fn validate_command_artifact(root: &Path, item: &Value, label: &str) -> Result<(), String> {
    let path = item
        .get("artifact_path")
        .and_then(Value::as_str)
        .unwrap_or("");
    let got = item
        .get("artifact_digest")
        .and_then(Value::as_str)
        .unwrap_or("");
    validate_path_digest(root, path, got, label)
}

fn placeholder_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    ["placeholder", "mock", "dummy", "fixture"]
        .iter()
        .any(|token| lower.contains(token))
}

fn reject_symlink_component(root: &Path, rel: &Path, label: &str) -> Result<(), String> {
    let mut cursor = root.to_path_buf();
    for name in rel.iter() {
        cursor.push(name);
        if std::fs::symlink_metadata(&cursor)
            .map(|meta| meta.file_type().is_symlink())
            .unwrap_or(false)
        {
            return Err(format!("{label} uses symlink artifact path"));
        }
    }
    Ok(())
}

fn ensure_inside_package(root: &Path, canonical: &Path, label: &str) -> Result<(), String> {
    if canonical.strip_prefix(root).is_err() {
        return Err(format!("{label} escapes package root"));
    }
    Ok(())
}

pub(crate) fn package_root_result(
    result: std::io::Result<std::path::PathBuf>,
) -> Result<std::path::PathBuf, String> {
    result.map_err(|err| format!("package root unavailable: {err}"))
}

pub(crate) fn package_artifact_canonical_result(
    label: &str,
    path: &str,
    result: std::io::Result<std::path::PathBuf>,
) -> Result<std::path::PathBuf, String> {
    result.map_err(|_| format!("{label} artifact missing: {path}"))
}

pub(crate) fn package_artifact_metadata_result(
    label: &str,
    path: &str,
    result: std::io::Result<Metadata>,
) -> Result<Metadata, String> {
    result.map_err(|_| format!("{label} artifact missing: {path}"))
}

pub(crate) fn package_artifact_digest_result(
    result: Result<String, String>,
) -> Result<String, String> {
    result
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn package_canonical_escape_guard_is_testable() {
        let root = Path::new("/package/root");
        assert!(
            super::ensure_inside_package(root, Path::new("/package/root/a.json"), "proof").is_ok()
        );
        assert!(
            super::ensure_inside_package(root, Path::new("/outside/a.json"), "proof")
                .expect_err("outside path rejected")
                .contains("escapes package root")
        );
    }
}
