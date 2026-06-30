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
    use std::path::Path;

    fn write_manifest(root: &Path) {
        let manifest = r#"{"resources":["docs/source.md"]}"#;
        std::fs::write(root.join("plugin-manifest-draft.json"), manifest).expect("manifest");
    }

    #[test]
    fn isolated_root_copies_package_files_and_removes_worker_writes() {
        let root = crate::self_tests::boundaries::support::temp_root("red-fixture-isolation");
        std::fs::create_dir_all(root.join("docs")).expect("docs");
        std::fs::create_dir_all(root.join("validation_artifacts/ultragoal-audit"))
            .expect("artifact dir");
        write_manifest(&root);
        std::fs::write(root.join("docs/source.md"), "ok").expect("source");
        let isolated = with_isolated_root(&root, "row/one", &[], |temp| {
            assert!(temp.join("docs/source.md").is_file());
            let worker = temp.join("validation_artifacts/ultragoal-audit/worker.txt");
            std::fs::write(&worker, "isolated").expect("worker write");
            temp.to_path_buf()
        })
        .expect("isolated root");
        let leaked_worker = root.join("validation_artifacts/ultragoal-audit/worker.txt");
        assert!(!leaked_worker.exists());
        assert!(!isolated.exists());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn isolated_root_edges_reject_invalid_paths_and_map_copy_errors() {
        let root = crate::self_tests::boundaries::support::temp_root("red-fixture-isolation-edges");
        std::fs::create_dir_all(root.join("docs")).expect("docs");
        write_manifest(&root);
        std::fs::write(root.join("docs/source.md"), "ok").expect("source");

        let isolated = with_isolated_root(&root, "", &["docs/source.md"], |temp| {
            assert!(temp.join("docs/source.md").is_file());
            let temp_name = temp.file_name().expect("temp leaf").to_string_lossy();
            assert!(temp_name.contains("invalid-row"));
            temp.to_path_buf()
        })
        .expect("isolated root");
        assert!(!isolated.exists());
        let missing_extra =
            with_isolated_root(&root, "missing-extra", &["docs/missing.md"], |temp| {
                assert!(!temp.join("docs/missing.md").exists());
                temp.to_path_buf()
            })
            .expect("missing extra skipped");
        assert!(!missing_extra.exists());
        assert_eq!(super::sanitized_row_id("row-one_ok"), "row-one_ok");

        let invalid_path = with_isolated_root(&root, "bad", &["../escape.json"], |_| ());
        let invalid_error = invalid_path.expect_err("invalid extra path");
        assert!(invalid_error.contains("red_fixture_isolated_root_path_invalid"));
        assert!(super::skip_dir(&root, Path::new("/outside-root")));
        let copy_error = super::copy_file(&root.join("docs/source.md"), Path::new(""))
            .expect_err("empty destination fails");
        assert!(copy_error.contains("red_fixture_isolated_root_copy_failed"));
        let blocked = crate::self_tests::boundaries::support::temp_root("red-fixture-blocked");
        std::fs::create_dir_all(&blocked).expect("blocked root");
        std::fs::write(blocked.join("target"), "not a dir").expect("target file");
        let ce = with_isolated_root(&blocked, "blocked", &[], |_| ()).expect_err("blocked target");
        assert!(ce.contains("red_fixture_isolated_root_create_failed"));
        let dest = root.join("dest");
        std::fs::create_dir_all(&dest).expect("dest");
        std::fs::write(dest.join("docs"), "not a dir").expect("dest docs file");
        let mkdir_error = super::copy_package_dirs(&root, &dest).expect_err("mkdir fails");
        assert!(mkdir_error.contains("red_fixture_isolated_root_mkdir_failed"));
        let parent_file = root.join("parent-file");
        std::fs::write(&parent_file, "not a dir").expect("parent file");
        let cm = super::copy_file(&root.join("docs/source.md"), &parent_file.join("out"))
            .expect_err("copy parent mkdir fails");
        assert!(cm.contains("red_fixture_isolated_root_mkdir_failed"));
        let rel_error = super::rel_path(&root, Path::new("/outside-root")).expect_err("rel fails");
        assert!(rel_error.contains("red_fixture_isolated_root_rel_failed"));
        std::fs::remove_dir_all(blocked).expect("cleanup blocked");
        let walk_error = super::copy_package_dirs(&root.join("missing"), &root.join("walk-dest"))
            .expect_err("walk");
        assert!(walk_error.contains("red_fixture_isolated_root_walk_failed"));
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
