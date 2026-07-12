use serde_json::Value;
#[cfg(unix)]
use std::ffi::{CString, OsStr, OsString};
#[cfg(unix)]
use std::fs::{File, OpenOptions};
#[cfg(unix)]
use std::io::Read;
#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
#[cfg(unix)]
use std::path::Component;
use std::path::Path;

#[path = "artifact/unique_json.rs"]
mod unique_json;
#[cfg(unix)]
mod unix_entry;

#[cfg(unix)]
use unix_entry::{
    Snapshot, directory_identity, directory_identity_at, file_snapshot, initial_file_snapshot,
    initial_path_snapshot, path_directory_identity,
};

const MAX_ANCHOR_BYTES: u64 = 4 * 1024 * 1024;
pub(super) const MAX_RELATIVE_PATH_BYTES: usize = 4096;
pub(super) const MAX_COMPONENTS: usize = 64;
const CHANGED: &str = "review_round_anchor_changed_during_read";
const MALFORMED: &str = "review_round_anchor_malformed";
const NOT_REGULAR: &str = "review_round_anchor_not_regular";
const PATH_INVALID: &str = "review_round_anchor_path_invalid";
const TOO_LARGE: &str = "review_round_anchor_too_large";
const UNAVAILABLE: &str = "review_round_anchor_unavailable";

pub(crate) struct JsonArtifact {
    pub(crate) value: Value,
    pub(crate) digest: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ReadStage {
    BeforeRootOpen,
    RootOpened,
    BeforeAncestorOpen(usize),
    AncestorOpened(usize),
    BeforeLeafOpen,
    LeafOpened,
    AfterRead,
}

pub(crate) fn read(root: &Path, relative: &str) -> Result<JsonArtifact, String> {
    read_inner(root, relative, |_| {})
}

#[cfg(test)]
pub(super) fn read_with_hook<F>(
    root: &Path,
    relative: &str,
    hook: F,
) -> Result<JsonArtifact, String>
where
    F: FnMut(ReadStage),
{
    read_inner(root, relative, hook)
}

#[cfg(unix)]
fn read_inner<F>(root: &Path, relative: &str, mut hook: F) -> Result<JsonArtifact, String>
where
    F: FnMut(ReadStage),
{
    let names = component_names(relative)?;
    if root.canonicalize().map_err(|_| PATH_INVALID)? != root {
        return Err(PATH_INVALID.to_string());
    }
    let (leaf, parents) = names.split_last().ok_or_else(|| PATH_INVALID.to_string())?;
    let before_root = path_directory_identity(root).map_err(|_| PATH_INVALID)?;
    hook(ReadStage::BeforeRootOpen);
    let root_file = open_root(root).map_err(|_| PATH_INVALID)?;
    let root_identity = directory_identity(&root_file).map_err(|_| PATH_INVALID)?;
    if root_identity != before_root {
        return Err(CHANGED.to_string());
    }
    hook(ReadStage::RootOpened);

    let mut directories = vec![root_file];
    let mut identities = vec![root_identity];
    for (index, name) in parents.iter().enumerate() {
        let expected = directory_identity_at(
            directories.last().ok_or_else(|| PATH_INVALID.to_string())?,
            name,
        )?;
        hook(ReadStage::BeforeAncestorOpen(index));
        let directory = open_at(
            directories.last().ok_or_else(|| PATH_INVALID.to_string())?,
            name,
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
        .map_err(initial_open_error)?;
        let identity = directory_identity(&directory).map_err(|_| PATH_INVALID)?;
        if identity != expected {
            return Err(CHANGED.to_string());
        }
        identities.push(identity);
        directories.push(directory);
        hook(ReadStage::AncestorOpened(index));
    }

    let parent = directories.last().ok_or_else(|| PATH_INVALID.to_string())?;
    let expected = initial_path_snapshot(parent, leaf)?;
    hook(ReadStage::BeforeLeafOpen);
    let mut file = open_at(
        parent,
        leaf,
        libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
    )
    .map_err(initial_open_error)?;
    let before = initial_file_snapshot(&file)?;
    if before != expected {
        return Err(CHANGED.to_string());
    }
    hook(ReadStage::LeafOpened);
    let mut bytes = Vec::with_capacity(usize::try_from(before.length).unwrap_or(0));
    file.by_ref()
        .take(MAX_ANCHOR_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| UNAVAILABLE.to_string())?;
    if bytes.len() as u64 > MAX_ANCHOR_BYTES {
        return Err(TOO_LARGE.to_string());
    }
    hook(ReadStage::AfterRead);
    if file_snapshot(&file).map_err(|_| CHANGED)? != before {
        return Err(CHANGED.to_string());
    }
    revalidate(root, &directories, &identities, parents, leaf, before)?;
    let value = unique_json::parse(&bytes).map_err(|_| MALFORMED.to_string())?;
    Ok(JsonArtifact {
        value,
        digest: crate::digest::bytes(&bytes),
    })
}

#[cfg(not(unix))]
fn read_inner<F>(_: &Path, _: &str, _: F) -> Result<JsonArtifact, String>
where
    F: FnMut(ReadStage),
{
    Err(PATH_INVALID.to_string())
}

#[cfg(unix)]
fn component_names(relative: &str) -> Result<Vec<OsString>, String> {
    if relative.as_bytes().len() > MAX_RELATIVE_PATH_BYTES {
        return Err(PATH_INVALID.to_string());
    }
    let mut names = Vec::new();
    for part in Path::new(relative).components() {
        if names.len() == MAX_COMPONENTS {
            return Err(PATH_INVALID.to_string());
        }
        match part {
            Component::Normal(name) if !name.as_bytes().is_empty() => {
                names.push(name.to_os_string())
            }
            _ => return Err(PATH_INVALID.to_string()),
        }
    }
    Ok(names)
}

#[cfg(unix)]
fn open_root(root: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
    options.open(root)
}

#[cfg(unix)]
fn open_at(parent: &File, name: &OsStr, flags: i32) -> std::io::Result<File> {
    let name = CString::new(name.as_bytes())?;
    let descriptor = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

#[cfg(unix)]
fn initial_open_error(error: std::io::Error) -> String {
    if error.kind() == std::io::ErrorKind::InvalidInput {
        return PATH_INVALID.to_string();
    }
    match error.raw_os_error() {
        Some(code) if code == libc::ELOOP || code == libc::ENOTDIR => PATH_INVALID.to_string(),
        _ => UNAVAILABLE.to_string(),
    }
}

#[cfg(unix)]
fn revalidate(
    root: &Path,
    directories: &[File],
    identities: &[(u64, u64)],
    parents: &[OsString],
    leaf: &OsStr,
    before: Snapshot,
) -> Result<(), String> {
    if path_directory_identity(root).map_err(|_| CHANGED)? != identities[0] {
        return Err(CHANGED.to_string());
    }
    for (index, name) in parents.iter().enumerate() {
        let reopened = open_at(
            &directories[index],
            name,
            libc::O_RDONLY
                | libc::O_DIRECTORY
                | libc::O_CLOEXEC
                | libc::O_NOFOLLOW
                | libc::O_NONBLOCK,
        )
        .map_err(|_| CHANGED)?;
        if directory_identity(&reopened).map_err(|_| CHANGED)? != identities[index + 1] {
            return Err(CHANGED.to_string());
        }
    }
    let parent = directories.last().ok_or_else(|| CHANGED.to_string())?;
    let reopened = open_at(
        parent,
        leaf,
        libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
    )
    .map_err(|_| CHANGED)?;
    if initial_file_snapshot(&reopened).map_err(|_| CHANGED)? != before {
        return Err(CHANGED.to_string());
    }
    Ok(())
}
