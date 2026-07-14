use sha2::{Digest, Sha256};
use std::fs::{self, DirEntry};
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use crate::routine_work::{RepoPath, RoutineError, RoutineErrorId};

const ENTRY_LIMIT: usize = 100_000;
const DEPTH_LIMIT: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct WorktreeShape(String);

impl WorktreeShape {
    pub(super) fn capture(root: &Path) -> Result<Self, RoutineError> {
        let mut hasher = Sha256::new();
        let mut stack = vec![(root.to_path_buf(), 0_usize)];
        let mut count = 0_usize;
        while let Some((directory, depth)) = stack.pop() {
            if depth > DEPTH_LIMIT {
                return Err(limit("worktree-depth-limit-exceeded"));
            }
            let mut entries = read_entries(&directory)?;
            entries.sort_by_key(DirEntry::file_name);
            for entry in entries.into_iter().rev() {
                let path = entry.path();
                let relative = path
                    .strip_prefix(root)
                    .map_err(|_| capture("worktree-entry-escape"))?;
                if depth == 0 && relative == Path::new(".git") {
                    continue;
                }
                count = count.saturating_add(1);
                if count > ENTRY_LIMIT {
                    return Err(limit("worktree-entry-limit-exceeded"));
                }
                let text = relative
                    .to_str()
                    .ok_or_else(|| unsupported("worktree-path-not-utf8"))?;
                RepoPath::parse(text.to_owned())?;
                let metadata = fs::symlink_metadata(&path)
                    .map_err(|_| capture("worktree-entry-metadata-failed"))?;
                let kind = classify(&metadata)?;
                update_identity(&mut hasher, text, kind, &metadata);
                if metadata.is_dir() {
                    stack.push((path, depth + 1));
                }
            }
        }
        Ok(Self(format!("sha256:{:x}", hasher.finalize())))
    }
}

fn read_entries(directory: &Path) -> Result<Vec<DirEntry>, RoutineError> {
    fs::read_dir(directory)
        .map_err(|_| capture("worktree-directory-read-failed"))?
        .map(|entry| entry.map_err(|_| capture("worktree-entry-read-failed")))
        .collect()
}

fn classify(metadata: &fs::Metadata) -> Result<u8, RoutineError> {
    if metadata.file_type().is_symlink() {
        return Err(unsupported("worktree-symlink-not-supported"));
    }
    if metadata.is_dir() {
        return Ok(b'd');
    }
    if metadata.is_file() {
        #[cfg(unix)]
        if metadata.nlink() != 1 {
            return Err(unsupported("worktree-hardlink-not-supported"));
        }
        return Ok(b'f');
    }
    Err(unsupported("worktree-special-entry-not-supported"))
}

fn update_identity(hasher: &mut Sha256, path: &str, kind: u8, metadata: &fs::Metadata) {
    hasher.update((path.len() as u64).to_be_bytes());
    hasher.update(path.as_bytes());
    hasher.update([kind]);
    hasher.update(metadata.len().to_be_bytes());
    #[cfg(unix)]
    {
        hasher.update(metadata.dev().to_be_bytes());
        hasher.update(metadata.ino().to_be_bytes());
        hasher.update(metadata.mode().to_be_bytes());
        hasher.update(metadata.nlink().to_be_bytes());
        hasher.update(metadata.mtime().to_be_bytes());
        hasher.update(metadata.mtime_nsec().to_be_bytes());
    }
    #[cfg(not(unix))]
    {
        hasher.update([u8::from(metadata.permissions().readonly())]);
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                hasher.update(duration.as_nanos().to_be_bytes());
            }
        }
    }
}

fn capture(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::CaptureFailed, cause, None)
}

fn unsupported(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::UnsupportedEntry, cause, None)
}

fn limit(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::CaptureLimit, cause, None)
}
