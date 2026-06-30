use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

pub(super) fn with_isolated_root<T>(
    source_root: &Path,
    row_id: &str,
    extra_files: &[&str],
    action: impl FnOnce(&Path) -> T,
) -> Result<T, String> {
    let guard = IsolatedRoot::create(source_root, row_id, extra_files)?;
    Ok(action(&guard.path))
}

pub(super) fn has_filesystem_fixtures(packet: &Value) -> bool {
    packet
        .get("filesystem_fixtures")
        .and_then(Value::as_array)
        .is_some_and(|fixtures| !fixtures.is_empty())
}

struct IsolatedRoot {
    path: PathBuf,
}

impl IsolatedRoot {
    fn create(source_root: &Path, row_id: &str, extra_files: &[&str]) -> Result<Self, String> {
        let path = temp_path(source_root, row_id);
        std::fs::create_dir_all(&path)
            .map_err(|err| format!("red_fixture_isolated_root_create_failed: {err}"))?;
        copy_package_dirs(source_root, &path)?;
        copy_package_files(source_root, &path, extra_files)?;
        Ok(Self { path })
    }
}

impl Drop for IsolatedRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn temp_path(source_root: &Path, row_id: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let index = COUNTER.fetch_add(1, Ordering::Relaxed);
    source_root
        .join("target")
        .join("ultragoal-red-fixtures")
        .join(format!(
            "{}-{}-{stamp}-{index}",
            std::process::id(),
            sanitized_row_id(row_id)
        ))
}

fn sanitized_row_id(row_id: &str) -> String {
    let cleaned = row_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .take(64)
        .collect::<String>();
    if cleaned.is_empty() {
        "invalid-row".to_string()
    } else {
        cleaned
    }
}

fn copy_package_dirs(source_root: &Path, dest_root: &Path) -> Result<(), String> {
    for entry in WalkDir::new(source_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !skip_dir(source_root, entry.path()))
    {
        let entry = entry.map_err(|err| format!("red_fixture_isolated_root_walk_failed: {err}"))?;
        if entry.file_type().is_dir() {
            let rel = rel_path(source_root, entry.path())?;
            std::fs::create_dir_all(dest_root.join(rel))
                .map_err(|err| format!("red_fixture_isolated_root_mkdir_failed: {err}"))?;
        }
    }
    Ok(())
}

fn skip_dir(source_root: &Path, path: &Path) -> bool {
    let Ok(rel) = path.strip_prefix(source_root) else {
        return true;
    };
    let rel = rel.to_string_lossy().replace('\\', "/");
    rel == ".git"
        || rel == "target"
        || rel == "node_modules"
        || rel == ".pnpm-store"
        || rel.starts_with(".git/")
        || rel.starts_with("target/")
        || rel.starts_with("node_modules/")
        || rel.starts_with(".pnpm-store/")
}

fn copy_package_files(
    source_root: &Path,
    dest_root: &Path,
    extra_files: &[&str],
) -> Result<(), String> {
    let mut paths = BTreeSet::new();
    if let Ok(manifest) =
        crate::json_boundary::read_json(&source_root.join("plugin-manifest-draft.json"))
    {
        paths.extend(crate::package::inventory::inventory_paths(&manifest));
        paths.insert("plugin-manifest-draft.json".to_string());
    }
    paths.extend(extra_files.iter().map(|rel| rel.to_string()));
    for rel in paths {
        if let Some(error) = crate::package::inventory::package_path_error(source_root, &rel) {
            return Err(format!(
                "red_fixture_isolated_root_path_invalid: {rel}: {error}"
            ));
        }
        let source = source_root.join(&rel);
        if source.is_file() {
            copy_file(&source, &dest_root.join(&rel))?;
        }
    }
    Ok(())
}

fn copy_file(source: &Path, dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("red_fixture_isolated_root_mkdir_failed: {err}"))?;
    }
    std::fs::copy(source, dest)
        .map(|_| ())
        .map_err(|err| format!("red_fixture_isolated_root_copy_failed: {err}"))
}

fn rel_path(root: &Path, path: &Path) -> Result<PathBuf, String> {
    path.strip_prefix(root)
        .map(PathBuf::from)
        .map_err(|err| format!("red_fixture_isolated_root_rel_failed: {err}"))
}

#[cfg(test)]
mod tests {
    use super::with_isolated_root;
    use serde_json::json;

    #[test]
    fn isolated_root_copies_package_files_and_removes_worker_writes() {
        let root = crate::self_tests::boundaries::support::temp_root("red-fixture-isolation");
        std::fs::create_dir_all(root.join("docs")).expect("docs");
        std::fs::create_dir_all(root.join("validation_artifacts/ultragoal-audit"))
            .expect("artifact dir");
        crate::json_boundary::write_json(
            &root.join("plugin-manifest-draft.json"),
            &json!({"resources":["docs/source.md"]}),
        )
        .expect("manifest");
        std::fs::write(root.join("docs/source.md"), "ok").expect("source");
        let isolated = with_isolated_root(&root, "row/one", &[], |temp| {
            assert!(temp.join("docs/source.md").is_file());
            let worker = temp.join("validation_artifacts/ultragoal-audit/worker.txt");
            std::fs::write(&worker, "isolated").expect("worker write");
            temp.to_path_buf()
        })
        .expect("isolated root");
        assert!(
            !root
                .join("validation_artifacts/ultragoal-audit/worker.txt")
                .exists()
        );
        assert!(!isolated.exists());
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
