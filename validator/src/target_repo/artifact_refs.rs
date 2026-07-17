use crate::digest;
use serde_json::Value;
use std::fs::Metadata;
use std::path::{Component, Path, PathBuf};

pub fn artifact_ref_error(repo: &Path, item: &Value, label: &str) -> Option<String> {
    resolve_artifact_path(repo, item, label).err()
}

pub fn read_artifact_json(repo: &Path, item: &Value, label: &str) -> Result<Value, String> {
    let path = resolve_artifact_path(repo, item, label)?;
    let bytes = artifact_bytes_result(label, digest::read_file_bytes(&path))?;
    artifact_json_result(label, serde_json::from_slice(&bytes))
}

fn resolve_artifact_path(repo: &Path, item: &Value, label: &str) -> Result<PathBuf, String> {
    let rel = item.get("path").and_then(Value::as_str).unwrap_or("");
    let got = item.get("digest").and_then(Value::as_str).unwrap_or("");
    if rel.is_empty() || got.is_empty() || got == digest::ZERO {
        return Err(format!("{label} lacks non-zero artifact digest"));
    }
    if ["placeholder", "mock", "dummy", "fixture"]
        .iter()
        .any(|token| rel.to_ascii_lowercase().contains(token))
    {
        return Err(format!("{label} uses placeholder artifact path: {rel}"));
    }
    let path = Path::new(rel);
    if path.is_absolute() {
        return Err(format!("{label} uses absolute artifact path"));
    }
    if path
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!("{label} escapes target repo"));
    }
    let root = target_root_result(repo.canonicalize())?;
    let joined = root.join(path);
    reject_symlink_component(&root, path, label)?;
    let resolved = artifact_canonical_result(label, rel, joined.canonicalize())?;
    ensure_inside_target_repo(&root, &resolved, label)?;
    let meta = artifact_metadata_result(label, rel, std::fs::symlink_metadata(&resolved))?;
    let file_type = meta.file_type();
    if file_type.is_symlink() || !file_type.is_file() {
        return Err(format!("{label} artifact is not a regular file: {rel}"));
    }
    match artifact_digest_result(digest::file(&resolved)) {
        Ok(actual) if actual == got => Ok(resolved),
        Ok(_) => Err(format!("{label} digest mismatch: {rel}")),
        Err(_) => Err(format!("{label} artifact unreadable: {rel}")),
    }
}

pub(crate) fn artifact_bytes_result(
    label: &str,
    result: Result<Vec<u8>, String>,
) -> Result<Vec<u8>, String> {
    result.map_err(|err| format!("{label} artifact unreadable: {err}"))
}

pub(crate) fn artifact_json_result(
    label: &str,
    result: serde_json::Result<Value>,
) -> Result<Value, String> {
    result.map_err(|err| format!("{label} artifact json malformed: {err}"))
}

pub(crate) fn target_root_result(result: std::io::Result<PathBuf>) -> Result<PathBuf, String> {
    result.map_err(|err| format!("target repo root unavailable: {err}"))
}

pub(crate) fn artifact_canonical_result(
    label: &str,
    rel: &str,
    result: std::io::Result<PathBuf>,
) -> Result<PathBuf, String> {
    result.map_err(|_| format!("{label} artifact missing: {rel}"))
}

pub(crate) fn artifact_metadata_result(
    label: &str,
    rel: &str,
    result: std::io::Result<Metadata>,
) -> Result<Metadata, String> {
    result.map_err(|_| format!("{label} artifact missing: {rel}"))
}

pub(crate) fn artifact_digest_result(result: Result<String, String>) -> Result<String, String> {
    result
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

fn ensure_inside_target_repo(root: &Path, resolved: &Path, label: &str) -> Result<(), String> {
    if resolved.strip_prefix(root).is_err() {
        return Err(format!("{label} escapes target repo"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn target_artifact_escape_guard_is_testable() {
        let root = Path::new("/target/root");
        assert!(
            super::ensure_inside_target_repo(
                root,
                Path::new("/target/root/artifact.json"),
                "artifact"
            )
            .is_ok()
        );
        assert!(
            super::ensure_inside_target_repo(root, Path::new("/outside/artifact.json"), "artifact")
                .expect_err("outside target repo rejected")
                .contains("escapes target repo")
        );
    }
}
