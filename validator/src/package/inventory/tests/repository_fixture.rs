use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) const CANARY: &str = "SECRET_CANARY must never enter digest or error";

pub(super) fn root(label: &str) -> PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    fs::create_dir_all(&root).expect("root");
    root
}

pub(super) fn resource_root(label: &str, relative: &str, bytes: &[u8]) -> PathBuf {
    let root = root(label);
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("resource parent")).expect("resource directory");
    fs::write(&path, bytes).expect("resource");
    write_manifest(&root, &[relative]);
    root
}

pub(super) fn write_manifest(root: &Path, resources: &[&str]) {
    fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({"resources": resources})).expect("manifest bytes"),
    )
    .expect("manifest");
}

pub(super) fn sibling(root: &Path, suffix: &str) -> PathBuf {
    let name = format!(
        "{}-{suffix}",
        root.file_name().expect("root name").to_string_lossy()
    );
    root.parent().expect("root parent").join(name)
}

pub(super) fn metadata_snapshot(root: &Path) -> Vec<(PathBuf, u64, bool)> {
    let mut rows = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .map(Result::unwrap)
        .map(|entry| {
            let metadata = fs::symlink_metadata(entry.path()).expect("metadata");
            (
                entry.path().strip_prefix(root).unwrap().to_path_buf(),
                metadata.len(),
                metadata.file_type().is_symlink(),
            )
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}
