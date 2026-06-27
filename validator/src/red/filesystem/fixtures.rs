use serde_json::Value;
use std::path::{Component, Path, PathBuf};

#[derive(Debug)]
pub(crate) struct FilesystemGuard {
    cleanup_paths: Vec<PathBuf>,
}

impl Drop for FilesystemGuard {
    fn drop(&mut self) {
        for path in self.cleanup_paths.iter().rev() {
            let _ = std::fs::remove_file(path);
        }
    }
}

pub(crate) fn materialize(root: &Path, packet: &Value) -> Result<FilesystemGuard, String> {
    let mut cleanup_paths = Vec::new();
    let Some(fixtures) = packet.get("filesystem_fixtures").and_then(Value::as_array) else {
        return Ok(FilesystemGuard { cleanup_paths });
    };
    for fixture in fixtures {
        materialize_one(root, fixture, &mut cleanup_paths)?;
    }
    Ok(FilesystemGuard { cleanup_paths })
}

fn materialize_one(
    root: &Path,
    fixture: &Value,
    cleanup_paths: &mut Vec<PathBuf>,
) -> Result<(), String> {
    match fixture.get("kind").and_then(Value::as_str) {
        Some("symlink") => materialize_symlink(root, fixture, cleanup_paths),
        Some("file") => materialize_file(root, fixture, cleanup_paths),
        _ => Err("red_filesystem_fixture_kind_unknown".to_string()),
    }
}

fn materialize_file(
    root: &Path,
    fixture: &Value,
    cleanup_paths: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let rel = fixture
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| "red_filesystem_fixture_path_missing".to_string())?;
    let contents = fixture
        .get("contents")
        .and_then(Value::as_str)
        .ok_or_else(|| "red_filesystem_fixture_contents_missing".to_string())?;
    let path = contained_normal_path(root, rel)?;
    if std::fs::symlink_metadata(&path).is_ok() {
        return Err("red_filesystem_fixture_path_exists".to_string());
    }
    let parent = path.parent().unwrap_or(root);
    if !parent.is_dir() {
        return Err("red_filesystem_fixture_parent_missing".to_string());
    }
    std::fs::write(&path, contents)
        .map_err(|err| format!("red_filesystem_fixture_file_write_failed: {err}"))?;
    cleanup_paths.push(path);
    Ok(())
}

fn materialize_symlink(
    root: &Path,
    fixture: &Value,
    cleanup_paths: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let link_rel = fixture
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| "red_filesystem_fixture_path_missing".to_string())?;
    let target_rel = fixture
        .get("target")
        .and_then(Value::as_str)
        .ok_or_else(|| "red_filesystem_fixture_target_missing".to_string())?;
    let link_path = contained_normal_path(root, link_rel)?;
    if std::fs::symlink_metadata(&link_path).is_ok() {
        return Err("red_filesystem_fixture_path_exists".to_string());
    }
    let parent = link_path.parent().unwrap_or(root);
    let target_path = parent.join(target_rel);
    let target_abs = target_path
        .canonicalize()
        .map_err(|err| format!("red_filesystem_fixture_target_invalid: {err}"))?;
    let root_abs = root
        .canonicalize()
        .map_err(|err| format!("root invalid: {err}"))?;
    if target_abs.strip_prefix(&root_abs).is_err() {
        return Err("red_filesystem_fixture_target_escapes_root".to_string());
    }
    create_symlink(target_rel, &link_path)?;
    cleanup_paths.push(link_path);
    Ok(())
}

fn contained_normal_path(root: &Path, rel: &str) -> Result<PathBuf, String> {
    if rel.is_empty() || Path::new(rel).is_absolute() {
        return Err("red_filesystem_fixture_path_invalid".to_string());
    }
    if Path::new(rel)
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("red_filesystem_fixture_path_invalid".to_string());
    }
    Ok(root.join(rel))
}

#[cfg(unix)]
fn create_symlink(target: &str, link: &Path) -> Result<(), String> {
    std::os::unix::fs::symlink(target, link)
        .map_err(|err| format!("red_filesystem_fixture_symlink_failed: {err}"))
}

#[cfg(not(unix))]
fn create_symlink(_target: &str, _link: &Path) -> Result<(), String> {
    Err("red_filesystem_fixture_symlink_unsupported".to_string())
}
