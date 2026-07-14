use super::types::InventoryError;
use crate::context::ReadSession;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

const MAX_WALK_DEPTH: usize = 64;
const MAX_WALK_ENTRIES: usize = 100_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CollectionEntryKind {
    Regular { single_link: bool },
    Directory,
    Symlink,
    Special,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CollectionEntry {
    pub(crate) path: PathBuf,
    pub(crate) kind: CollectionEntryKind,
}

#[derive(Clone, Copy)]
enum Profile {
    Collection,
    Repository,
}

#[cfg(unix)]
#[derive(Eq, PartialEq)]
struct DirectorySnapshot {
    path: PathBuf,
    device: u64,
    inode: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(unix)]
fn directory_snapshot(path: PathBuf, metadata: &fs::Metadata) -> DirectorySnapshot {
    DirectorySnapshot {
        path,
        device: metadata.dev(),
        inode: metadata.ino(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    }
}

fn skipped(entry: &walkdir::DirEntry, profile: Profile) -> bool {
    if !matches!(profile, Profile::Repository) || entry.depth() == 0 || !entry.file_type().is_dir()
    {
        return false;
    }
    matches!(
        entry.file_name().to_string_lossy().as_ref(),
        ".git" | "target" | "validation_artifacts" | ".codex-worktree" | "node_modules"
    )
}

fn walk(
    reads: &ReadSession,
    start: &Path,
    profile: Profile,
) -> Result<Vec<CollectionEntry>, InventoryError> {
    reads
        .observe_presence_parent(start)
        .map_err(|error| InventoryError::Io {
            path: start.to_path_buf(),
            message: error.to_string(),
        })?;
    if !start.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    #[cfg(unix)]
    let mut directories = Vec::new();
    let walker = walkdir::WalkDir::new(start)
        .follow_links(false)
        .max_depth(MAX_WALK_DEPTH + 1)
        .into_iter();
    for entry in walker.filter_entry(|entry| !skipped(entry, profile)) {
        reads.charge_entry().map_err(|error| InventoryError::Io {
            path: start.to_path_buf(),
            message: error.to_string(),
        })?;
        let entry = entry.map_err(|error| InventoryError::Io {
            path: error
                .path()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| start.to_path_buf()),
            message: error.to_string(),
        })?;
        if entry.depth() > MAX_WALK_DEPTH {
            return Err(InventoryError::InvalidRegistry(format!(
                "inventory walk exceeds depth {MAX_WALK_DEPTH}"
            )));
        }
        let kind = if entry.file_type().is_dir() {
            #[cfg(unix)]
            {
                let metadata = entry.metadata().map_err(|error| InventoryError::Io {
                    path: entry.path().to_path_buf(),
                    message: error.to_string(),
                })?;
                let pinned =
                    reads
                        .pin_directory(entry.path())
                        .map_err(|error| InventoryError::Io {
                            path: entry.path().to_path_buf(),
                            message: error.to_string(),
                        })?;
                if pinned != (metadata.dev(), metadata.ino()) {
                    return Err(InventoryError::Io {
                        path: entry.path().to_path_buf(),
                        message: "enumerated directory does not match its pinned descriptor"
                            .to_owned(),
                    });
                }
                directories.push(directory_snapshot(entry.path().to_path_buf(), &metadata));
            }
            CollectionEntryKind::Directory
        } else if entry.file_type().is_file() {
            let metadata = entry.metadata().map_err(|error| InventoryError::Io {
                path: entry.path().to_path_buf(),
                message: error.to_string(),
            })?;
            #[cfg(unix)]
            let single_link = metadata.nlink() == 1;
            #[cfg(not(unix))]
            let single_link = false;
            CollectionEntryKind::Regular { single_link }
        } else if entry.file_type().is_symlink() {
            CollectionEntryKind::Symlink
        } else {
            CollectionEntryKind::Special
        };
        if entry.depth() != 0 || !matches!(kind, CollectionEntryKind::Directory) {
            entries.push(CollectionEntry {
                path: entry.into_path(),
                kind,
            });
            if entries.len() > MAX_WALK_ENTRIES {
                return Err(InventoryError::InvalidRegistry(format!(
                    "inventory walk exceeds {MAX_WALK_ENTRIES} entries"
                )));
            }
        }
    }
    #[cfg(unix)]
    for before in directories {
        let metadata = fs::metadata(&before.path).map_err(|error| InventoryError::Io {
            path: before.path.clone(),
            message: error.to_string(),
        })?;
        if before != directory_snapshot(before.path.clone(), &metadata) {
            return Err(InventoryError::Io {
                path: before.path,
                message: "directory identity changed during inventory walk".to_owned(),
            });
        }
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

pub(crate) fn collection_entries(
    reads: &ReadSession,
    start: &Path,
) -> Result<Vec<CollectionEntry>, InventoryError> {
    walk(reads, start, Profile::Collection)
}

pub(crate) fn collection_files(
    reads: &ReadSession,
    start: &Path,
) -> Result<Vec<PathBuf>, InventoryError> {
    Ok(collection_entries(reads, start)?
        .into_iter()
        .filter(|entry| {
            matches!(
                entry.kind,
                CollectionEntryKind::Regular { .. } | CollectionEntryKind::Symlink
            )
        })
        .map(|entry| entry.path)
        .collect())
}

pub(crate) fn repository_files(
    reads: &ReadSession,
    root: &Path,
) -> Result<Vec<PathBuf>, InventoryError> {
    Ok(walk(reads, root, Profile::Repository)?
        .into_iter()
        .filter(|entry| {
            matches!(
                entry.kind,
                CollectionEntryKind::Regular { .. } | CollectionEntryKind::Symlink
            )
        })
        .map(|entry| entry.path)
        .collect())
}

#[cfg(test)]
mod tests;
