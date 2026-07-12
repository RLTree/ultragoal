use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

use super::super::{RepoPath, RoutineError, RoutineErrorId};

const FILE_LIMIT: u64 = 16 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RootAnchor {
    canonical: PathBuf,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

impl RootAnchor {
    pub(super) fn capture(root: &Path) -> Result<Self, RoutineError> {
        let metadata = fs::symlink_metadata(root).map_err(|_| io_error("root-metadata-failed"))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(unsupported("root-not-anchorable"));
        }
        let canonical = root
            .canonicalize()
            .map_err(|_| io_error("root-canonicalization-failed"))?;
        if canonical != root {
            return Err(unsupported("root-alias-not-accepted"));
        }
        Ok(Self {
            canonical,
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
        })
    }

    pub(super) fn revalidate(&self, root: &Path) -> Result<(), RoutineError> {
        let current = Self::capture(root)?;
        if &current != self {
            return Err(RoutineError::new(
                RoutineErrorId::ConcurrentMutation,
                "worktree-root-identity-changed",
                None,
            ));
        }
        Ok(())
    }
}

pub(super) fn content_identity(root: &Path, relative: &RepoPath) -> Result<String, RoutineError> {
    let root_anchor = RootAnchor::capture(root)?;
    let path = root.join(relative.as_str());
    validate_ancestors(root, relative)?;
    let before = fs::symlink_metadata(&path).map_err(|_| io_error("entry-metadata-failed"))?;
    validate_regular(&before)?;
    let canonical = path
        .canonicalize()
        .map_err(|_| io_error("entry-canonicalization-failed"))?;
    if canonical != path || !canonical.starts_with(root) {
        return Err(unsupported("entry-alias-or-escape"));
    }
    let first = digest_open(&path, &before)?;
    let between = fs::symlink_metadata(&path).map_err(|_| io_error("entry-revalidation-failed"))?;
    if !same_identity(&before, &between) {
        return Err(mutated("entry-identity-changed"));
    }
    let second = digest_open(&path, &between)?;
    validate_ancestors(root, relative)?;
    root_anchor.revalidate(root)?;
    if first != second {
        return Err(mutated("entry-content-changed"));
    }
    Ok(first)
}

fn validate_ancestors(root: &Path, relative: &RepoPath) -> Result<(), RoutineError> {
    let mut current = root.to_path_buf();
    let components = relative.as_str().split('/').collect::<Vec<_>>();
    for component in components.iter().take(components.len().saturating_sub(1)) {
        current.push(component);
        let metadata =
            fs::symlink_metadata(&current).map_err(|_| io_error("ancestor-metadata-failed"))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(unsupported("ancestor-not-directory"));
        }
    }
    Ok(())
}

fn digest_open(path: &Path, expected: &fs::Metadata) -> Result<String, RoutineError> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    let mut file = options
        .open(path)
        .map_err(|_| io_error("entry-open-failed"))?;
    let opened = file
        .metadata()
        .map_err(|_| io_error("opened-metadata-failed"))?;
    validate_regular(&opened)?;
    if !same_identity(expected, &opened) {
        return Err(mutated("opened-entry-identity-changed"));
    }
    digest_reader(&mut file, opened.len())
}

fn digest_reader(file: &mut File, length: u64) -> Result<String, RoutineError> {
    if length > FILE_LIMIT {
        return Err(RoutineError::new(
            RoutineErrorId::CaptureLimit,
            "dirty-entry-size-limit-exceeded",
            None,
        ));
    }
    let mut hasher = Sha256::new();
    let mut consumed = 0_u64;
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| io_error("entry-read-failed"))?;
        if read == 0 {
            break;
        }
        consumed = consumed.saturating_add(read as u64);
        if consumed > FILE_LIMIT {
            return Err(RoutineError::new(
                RoutineErrorId::CaptureLimit,
                "dirty-entry-size-limit-exceeded",
                None,
            ));
        }
        hasher.update(&buffer[..read]);
    }
    if consumed != length {
        return Err(mutated("entry-length-changed"));
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn validate_regular(metadata: &fs::Metadata) -> Result<(), RoutineError> {
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(unsupported("dirty-entry-not-regular"));
    }
    #[cfg(unix)]
    if metadata.nlink() != 1 {
        return Err(unsupported("dirty-entry-link-count-not-one"));
    }
    Ok(())
}

#[cfg(unix)]
fn same_identity(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.dev() == right.dev()
        && left.ino() == right.ino()
        && left.len() == right.len()
        && left.mode() == right.mode()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
}

#[cfg(not(unix))]
fn same_identity(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.len() == right.len()
        && left.permissions().readonly() == right.permissions().readonly()
        && left.modified().ok() == right.modified().ok()
}

fn io_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::CaptureFailed, cause, None)
}

fn unsupported(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::UnsupportedEntry, cause, None)
}

fn mutated(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::ConcurrentMutation, cause, None)
}
