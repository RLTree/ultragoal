use super::PackageEntryKind;
use crate::package::inventory::anchored::Session;
use std::collections::BTreeMap;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

const MAX_DEPTH: usize = 64;
const MAX_ENTRIES: usize = 100_000;
const LOCAL_PREFIXES: &[&str] = &[
    ".git/",
    ".codex-worktree/",
    ".ui-discipline/",
    ".pnpm-store/",
    "node_modules/",
    "target/",
    "validation_artifacts/",
    "state/codex-review-artifacts/",
    "state/codex-review-receipts.d/",
];

pub(super) fn capture(
    root: &Path,
    session: &mut Session,
) -> Result<BTreeMap<String, PackageEntryKind>, String> {
    session.verify_root_now()?;
    let mut out = BTreeMap::new();
    let walker = WalkDir::new(root)
        .follow_links(false)
        .max_depth(MAX_DEPTH + 1)
        .into_iter()
        .filter_entry(|entry| should_descend(root, entry));
    for (visited, entry) in walker.enumerate() {
        if visited >= MAX_ENTRIES {
            return Err("package snapshot tree exceeds its entry limit".to_string());
        }
        let entry = entry.map_err(|_| "package snapshot tree enumeration failed".to_string())?;
        if entry.depth() == 0 {
            continue;
        }
        if entry.depth() > MAX_DEPTH {
            return Err("package snapshot tree exceeds its depth limit".to_string());
        }
        let relative = relative(root, entry.path())?;
        if local_only(&relative) {
            continue;
        }
        let observed = session.observe_entry(&relative)?;
        let kind = if observed.regular {
            PackageEntryKind::Regular {
                single_link: observed.single_link,
            }
        } else if observed.directory {
            PackageEntryKind::Directory
        } else if observed.symlink {
            PackageEntryKind::Symlink
        } else {
            PackageEntryKind::Special
        };
        if !enumeration_matches(&entry, kind) {
            return Err("package snapshot tree changed during enumeration".to_string());
        }
        if out.insert(relative, kind).is_some() {
            return Err("package snapshot tree contains a duplicate path".to_string());
        }
    }
    session.verify_root_now()?;
    Ok(out)
}

fn should_descend(root: &Path, entry: &DirEntry) -> bool {
    entry.depth() == 0
        || relative(root, entry.path())
            .map(|path| !local_only(&path))
            .unwrap_or(true)
}

fn enumeration_matches(entry: &DirEntry, kind: PackageEntryKind) -> bool {
    let file_type = entry.file_type();
    match kind {
        PackageEntryKind::Regular { .. } => file_type.is_file(),
        PackageEntryKind::Directory => file_type.is_dir(),
        PackageEntryKind::Symlink => file_type.is_symlink(),
        PackageEntryKind::Special => {
            !file_type.is_file() && !file_type.is_dir() && !file_type.is_symlink()
        }
    }
}

fn relative(root: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(root)
        .ok()
        .and_then(Path::to_str)
        .map(str::to_owned)
        .ok_or_else(|| "package snapshot tree path is invalid".to_string())
}

fn local_only(relative: &str) -> bool {
    crate::package::inventory::builder_contract_resource_path(relative)
        || LOCAL_PREFIXES
            .iter()
            .any(|prefix| relative == prefix.trim_end_matches('/') || relative.starts_with(prefix))
}
