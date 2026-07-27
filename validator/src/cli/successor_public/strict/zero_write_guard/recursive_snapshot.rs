use super::scope_policy;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

const MAX_ENTRIES: usize = 250_000;
const MAX_HASHED_BYTES: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ZeroWriteSnapshot {
    rows: Vec<SnapshotRow>,
}

#[derive(Debug, Eq, PartialEq)]
struct SnapshotRow {
    relative_path: PathBuf,
    kind: EntryKind,
    byte_length: u64,
    content_sha256: String,
    unix_mode: Option<u32>,
    modified_ns: Option<u128>,
    changed: Option<(i64, i64)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EntryKind {
    Directory,
    ExcludedDirectory,
    Regular,
    Symlink,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ZeroWriteCaptureError {
    RootUnavailable,
    TreeUnavailable,
    UnsupportedObject,
    EntryLimitExceeded,
    ByteLimitExceeded,
    ConcurrentMutation,
}

struct CaptureBudget {
    entries: usize,
    hashed_bytes: u64,
}

pub(crate) fn capture(root: &Path) -> Result<ZeroWriteSnapshot, ZeroWriteCaptureError> {
    let root = root
        .canonicalize()
        .map_err(|_| ZeroWriteCaptureError::RootUnavailable)?;
    let metadata =
        fs::symlink_metadata(&root).map_err(|_| ZeroWriteCaptureError::RootUnavailable)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(ZeroWriteCaptureError::RootUnavailable);
    }
    let mut rows = Vec::new();
    let mut budget = CaptureBudget {
        entries: 0,
        hashed_bytes: 0,
    };
    capture_entry(&root, &root, &mut rows, &mut budget)?;
    Ok(ZeroWriteSnapshot { rows })
}

fn capture_entry(
    root: &Path,
    path: &Path,
    rows: &mut Vec<SnapshotRow>,
    budget: &mut CaptureBudget,
) -> Result<(), ZeroWriteCaptureError> {
    budget.entries = budget.entries.saturating_add(1);
    if budget.entries > MAX_ENTRIES {
        return Err(ZeroWriteCaptureError::EntryLimitExceeded);
    }
    let relative = path
        .strip_prefix(root)
        .map_err(|_| ZeroWriteCaptureError::TreeUnavailable)?;
    let before = fs::symlink_metadata(path).map_err(|_| ZeroWriteCaptureError::TreeUnavailable)?;
    let excluded = before.is_dir() && scope_policy::excluded(relative);
    let (kind, content_sha256) = if excluded {
        (EntryKind::ExcludedDirectory, crate::digest::bytes(&[]))
    } else if before.is_dir() {
        (EntryKind::Directory, crate::digest::bytes(&[]))
    } else if before.is_file() {
        budget.hashed_bytes = budget
            .hashed_bytes
            .checked_add(before.len())
            .ok_or(ZeroWriteCaptureError::ByteLimitExceeded)?;
        if budget.hashed_bytes > MAX_HASHED_BYTES {
            return Err(ZeroWriteCaptureError::ByteLimitExceeded);
        }
        let digest =
            crate::digest::file(path).map_err(|_| ZeroWriteCaptureError::ConcurrentMutation)?;
        (EntryKind::Regular, digest)
    } else if before.file_type().is_symlink() {
        let target = fs::read_link(path).map_err(|_| ZeroWriteCaptureError::TreeUnavailable)?;
        (
            EntryKind::Symlink,
            crate::digest::bytes(&path_bytes(&target)),
        )
    } else {
        return Err(ZeroWriteCaptureError::UnsupportedObject);
    };
    if before.is_dir() && !excluded {
        let mut children = fs::read_dir(path)
            .map_err(|_| ZeroWriteCaptureError::TreeUnavailable)?
            .map(|entry| entry.map(|value| value.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ZeroWriteCaptureError::TreeUnavailable)?;
        children.sort();
        for child in children {
            capture_entry(root, &child, rows, budget)?;
        }
    }
    let after =
        fs::symlink_metadata(path).map_err(|_| ZeroWriteCaptureError::ConcurrentMutation)?;
    if identity(&before) != identity(&after) {
        return Err(ZeroWriteCaptureError::ConcurrentMutation);
    }
    rows.push(row(relative, &after, kind, content_sha256));
    Ok(())
}

fn row(
    relative: &Path,
    metadata: &fs::Metadata,
    kind: EntryKind,
    content_sha256: String,
) -> SnapshotRow {
    SnapshotRow {
        relative_path: relative.to_path_buf(),
        kind,
        byte_length: metadata.len(),
        content_sha256,
        unix_mode: unix_mode(metadata),
        modified_ns: metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .map(|value| value.as_nanos()),
        changed: changed(metadata),
    }
}

#[cfg(unix)]
fn identity(metadata: &fs::Metadata) -> (u64, u64, u32, u64, u64, i64, i64, i64, i64) {
    (
        metadata.dev(),
        metadata.ino(),
        metadata.mode(),
        metadata.nlink(),
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec(),
        metadata.ctime(),
        metadata.ctime_nsec(),
    )
}

#[cfg(not(unix))]
fn identity(metadata: &fs::Metadata) -> (u64, bool, Option<u128>) {
    (
        metadata.len(),
        metadata.permissions().readonly(),
        metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .map(|value| value.as_nanos()),
    )
}

#[cfg(unix)]
fn unix_mode(metadata: &fs::Metadata) -> Option<u32> {
    Some(metadata.mode())
}
#[cfg(not(unix))]
fn unix_mode(_metadata: &fs::Metadata) -> Option<u32> {
    None
}

#[cfg(unix)]
fn changed(metadata: &fs::Metadata) -> Option<(i64, i64)> {
    Some((metadata.ctime(), metadata.ctime_nsec()))
}
#[cfg(not(unix))]
fn changed(_metadata: &fs::Metadata) -> Option<(i64, i64)> {
    None
}

fn path_bytes(path: &Path) -> Vec<u8> {
    #[cfg(unix)]
    {
        path.as_os_str().as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        path.to_string_lossy().as_bytes().to_vec()
    }
}
