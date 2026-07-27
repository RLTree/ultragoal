//! Candidate-bound protection for intentional Git deletions.

use super::{ContextError, ReadSession, query_git};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Component, PathBuf};

#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

/// Returns requested paths that a current Git candidate records as deleted or
/// unmerged. Rename detection is disabled so a renamed source stays protected.
pub(crate) fn planned_write_conflict_paths(
    reads: &ReadSession,
    requested: &[PathBuf],
) -> Result<Vec<PathBuf>, ContextError> {
    if requested.is_empty() {
        return Ok(Vec::new());
    }
    let requested = requested.iter().collect::<BTreeSet<_>>();
    let mut conflicts = BTreeSet::new();
    for arguments in [
        [
            "-c",
            "diff.renames=false",
            "diff",
            "--name-only",
            "--diff-filter=DU",
            "-z",
            "--",
        ]
        .as_slice(),
        [
            "-c",
            "diff.renames=false",
            "diff",
            "--cached",
            "--name-only",
            "--diff-filter=DU",
            "-z",
            "--",
        ]
        .as_slice(),
    ] {
        for path in query_git(reads, arguments)?
            .split(|byte| *byte == 0)
            .filter(|path| !path.is_empty())
            .map(relative_path)
        {
            let path = path?;
            if requested.contains(&path) {
                conflicts.insert(path);
            }
        }
    }
    Ok(conflicts.into_iter().collect())
}

fn relative_path(bytes: &[u8]) -> Result<PathBuf, ContextError> {
    #[cfg(unix)]
    let path = PathBuf::from(OsString::from_vec(bytes.to_vec()));
    #[cfg(not(unix))]
    let path = PathBuf::from(String::from_utf8_lossy(bytes).into_owned());
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ContextError::ConcurrentMutation(
            "Git returned a non-relative planned-write path".to_owned(),
        ));
    }
    Ok(path)
}
