use super::digest::add_framed;
use super::error::io_error;
use super::{ContextError, ReadSession, query_git};
use sha2::{Digest, Sha256};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[cfg(unix)]
use std::os::unix::ffi::{OsStrExt, OsStringExt};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const BRIEF_PATH: &str = "PRODUCT_SUCCESS_BRIEF.json";
const MAX_UNTRACKED_BYTES: u64 = 16 * 1024 * 1024;

/// Derives the current worktree identity for a Product Success Brief subject.
/// The brief is intentionally omitted so it can bind the work it describes
/// without creating a hash fixed point; every other tracked or untracked byte
/// remains identity-bearing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SubjectIdentity {
    pub(crate) digest: String,
    pub(crate) dirty: bool,
}

pub(crate) fn identity(reads: &ReadSession) -> Result<SubjectIdentity, ContextError> {
    reads.revalidate()?;
    let mut hasher = Sha256::new();
    add_framed(&mut hasher, b"ProductInceptionSubject-v1");
    let mut status = Vec::new();
    for (index, arguments) in [
        ["ls-files", "-s", "-z", "--", ".", exclude()].as_slice(),
        [
            "diff",
            "--binary",
            "--no-ext-diff",
            "--no-textconv",
            "--",
            ".",
            exclude(),
        ]
        .as_slice(),
        [
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--",
            ".",
            exclude(),
        ]
        .as_slice(),
    ]
    .into_iter()
    .enumerate()
    {
        let output = query_git(reads, arguments)?;
        if index == 2 {
            status = output.clone();
        }
        add_framed(&mut hasher, &output);
    }
    for relative in untracked_paths(reads)? {
        add_untracked(reads, &mut hasher, relative)?;
    }
    reads.revalidate()?;
    Ok(SubjectIdentity {
        digest: format!("sha256:{:x}", hasher.finalize()),
        dirty: !status.is_empty(),
    })
}

fn exclude() -> &'static str {
    ":(exclude)PRODUCT_SUCCESS_BRIEF.json"
}

fn untracked_paths(reads: &ReadSession) -> Result<Vec<PathBuf>, ContextError> {
    let listing = query_git(
        reads,
        [
            "ls-files",
            "--others",
            "--exclude-standard",
            "-z",
            "--",
            ".",
            exclude(),
        ]
        .as_slice(),
    )?;
    let mut paths = listing
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(checked_relative)
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    Ok(paths)
}

fn checked_relative(bytes: &[u8]) -> Result<PathBuf, ContextError> {
    let path = path_from_bytes(bytes.to_vec());
    if path == Path::new(BRIEF_PATH)
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(ContextError::PathDenied(
            "Git returned an invalid Product Inception path".to_owned(),
        ));
    }
    Ok(path)
}

fn path_from_bytes(bytes: Vec<u8>) -> PathBuf {
    #[cfg(unix)]
    {
        PathBuf::from(OsString::from_vec(bytes))
    }
    #[cfg(not(unix))]
    {
        PathBuf::from(String::from_utf8_lossy(&bytes).into_owned())
    }
}

fn os_bytes(value: &OsStr) -> Vec<u8> {
    #[cfg(unix)]
    {
        value.as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        value.to_string_lossy().as_bytes().to_vec()
    }
}

fn add_untracked(
    reads: &ReadSession,
    hasher: &mut Sha256,
    relative: PathBuf,
) -> Result<(), ContextError> {
    let path = reads.root().join(&relative);
    let metadata = fs::symlink_metadata(&path).map_err(|error| io_error(&path, error))?;
    add_framed(hasher, &os_bytes(relative.as_os_str()));
    #[cfg(unix)]
    hasher.update(metadata.permissions().mode().to_be_bytes());
    #[cfg(not(unix))]
    hasher.update([metadata.permissions().readonly() as u8]);
    hasher.update(metadata.len().to_be_bytes());
    if metadata.is_file() {
        add_framed(hasher, &reads.read_bounded(&path, MAX_UNTRACKED_BYTES)?);
    } else if metadata.file_type().is_symlink() {
        let target = fs::read_link(&path).map_err(|error| io_error(&path, error))?;
        add_framed(hasher, &os_bytes(target.as_os_str()));
    } else {
        return Err(ContextError::PathDenied(
            "Product Inception subject includes a special untracked path".to_owned(),
        ));
    }
    Ok(())
}
