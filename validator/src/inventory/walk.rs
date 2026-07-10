use super::types::InventoryError;
use crate::context::ReadSession;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

const MAX_WALK_DEPTH: usize = 64;
const MAX_WALK_FILES: usize = 100_000;

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
) -> Result<Vec<PathBuf>, InventoryError> {
    reads
        .observe_presence_parent(start)
        .map_err(|error| InventoryError::Io {
            path: start.to_path_buf(),
            message: error.to_string(),
        })?;
    if !start.exists() {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
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
        if entry.file_type().is_dir() {
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
        } else if entry.file_type().is_file() || entry.file_type().is_symlink() {
            paths.push(entry.into_path());
            if paths.len() > MAX_WALK_FILES {
                return Err(InventoryError::InvalidRegistry(format!(
                    "inventory walk exceeds {MAX_WALK_FILES} files"
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
    paths.sort();
    Ok(paths)
}

pub(crate) fn collection_files(
    reads: &ReadSession,
    start: &Path,
) -> Result<Vec<PathBuf>, InventoryError> {
    walk(reads, start, Profile::Collection)
}

pub(crate) fn repository_files(
    reads: &ReadSession,
    root: &Path,
) -> Result<Vec<PathBuf>, InventoryError> {
    walk(reads, root, Profile::Repository)
}
