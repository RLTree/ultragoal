use std::path::Path;
use walkdir::{DirEntry, WalkDir};

const LOCAL_TRANSIENT_OUTPUT_PREFIXES: &[&str] = &[
    "target/",
    ".codex-worktree/",
    ".ui-discipline/",
    ".git/",
    "node_modules/",
    ".pnpm-store/",
    "validation_artifacts/",
    "state/codex-review-artifacts/",
    "state/codex-review-receipts.d/",
];
const LOCAL_TRANSIENT_OUTPUT_COMPONENTS: &[&str] = &["target"];

pub fn actual_files(root: &Path) -> Result<Vec<String>, String> {
    walk_package_entries(root, |entry| entry.file_type().is_file())
}

pub(super) fn symlink_entries(root: &Path) -> Result<Vec<String>, String> {
    walk_package_entries(root, |entry| entry.file_type().is_symlink())
}

fn walk_package_entries(
    root: &Path,
    include: impl Fn(&walkdir::DirEntry) -> bool,
) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| should_descend(root, entry));
    for entry in walker {
        let entry = entry.map_err(|err| format!("walk failed: {err}"))?;
        if include(&entry) {
            let rel = rel_path(root, entry.path())?;
            if !local_only(&rel) {
                paths.push(rel);
            }
        }
    }
    paths.sort();
    Ok(paths)
}

fn should_descend(root: &Path, entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return true;
    }
    rel_path(root, entry.path())
        .map(|rel| !local_only(&rel))
        .unwrap_or(true)
}

fn local_only(rel: &str) -> bool {
    crate::package::inventory::builder_contract_resource_path(rel)
        || LOCAL_TRANSIENT_OUTPUT_COMPONENTS
            .iter()
            .any(|component| rel.split('/').any(|part| part == *component))
        || LOCAL_TRANSIENT_OUTPUT_PREFIXES
            .iter()
            .any(|prefix| local_prefix_match(rel, prefix))
}

fn local_prefix_match(rel: &str, prefix: &str) -> bool {
    let directory = prefix.trim_end_matches('/');
    rel == directory || rel.starts_with(prefix)
}

fn rel_path(root: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(root)
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .map_err(|err| format!("walk root strip failed: {err}"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn package_walk_prunes_local_transient_output_directories() {
        let root = std::env::temp_dir().join(format!(
            "ultragoal-package-walk-local-output-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("target/transient")).expect("target dir");
        std::fs::create_dir_all(root.join("validator/target/transient"))
            .expect("nested target dir");
        std::fs::create_dir_all(root.join("state/codex-review-artifacts"))
            .expect("review artifact dir");
        std::fs::create_dir_all(root.join("state/codex-review-receipts.d"))
            .expect("review receipt dir");
        std::fs::create_dir_all(root.join("src")).expect("src dir");
        std::fs::write(root.join("target/transient/output.txt"), "local").expect("target file");
        std::fs::write(
            root.join("validator/target/transient/output.txt"),
            "local nested",
        )
        .expect("nested target file");
        std::fs::write(
            root.join("state/codex-review-artifacts/review.txt"),
            "external debug",
        )
        .expect("review artifact file");
        std::fs::write(
            root.join("state/codex-review-receipts.d/review.jsonl"),
            "external debug\n",
        )
        .expect("review receipt file");
        std::fs::write(root.join("src/lib.rs"), "fn main() {}\n").expect("source file");

        let files = super::actual_files(&root).expect("actual files");
        let _ = std::fs::remove_dir_all(&root);

        assert_eq!(files, vec!["src/lib.rs".to_string()]);
        assert!(super::local_only("target"));
        assert!(super::local_only("target/transient/output.txt"));
        assert!(super::local_only("validator/target/transient/output.txt"));
        assert!(super::local_only("state/codex-review-artifacts/review.txt"));
        assert!(super::local_only(
            "state/codex-review-receipts.d/review.jsonl"
        ));
        assert!(!super::local_only("target-file.txt"));
    }

    #[test]
    fn package_walk_reports_paths_outside_walk_root() {
        assert!(
            super::rel_path(
                std::path::Path::new("/package/root"),
                std::path::Path::new("/outside/file")
            )
            .expect_err("outside path rejected")
            .contains("walk root strip failed")
        );
    }
}
