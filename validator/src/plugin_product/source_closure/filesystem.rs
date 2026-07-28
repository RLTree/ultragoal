use super::model::ClosureError;
use sha2::{Digest, Sha256};
#[cfg(not(unix))]
use std::fs;
use std::fs::Metadata;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[cfg(unix)]
#[path = "source_closure_filesystem_descriptor.rs"]
mod descriptor;
#[cfg(not(unix))]
mod descriptor {
    use super::{ClosureError, Path, PathBuf};
    use std::fs::File;

    pub(super) struct CheckedRoot {
        path: PathBuf,
    }
    impl CheckedRoot {
        pub(super) fn path(&self) -> &Path {
            &self.path
        }
    }
    pub(super) fn checked_root(_: &Path) -> Result<CheckedRoot, ClosureError> {
        Err(ClosureError::Unreadable)
    }
    pub(super) fn open_confined(_: &CheckedRoot, _: &Path) -> Result<File, ClosureError> {
        Err(ClosureError::Unreadable)
    }
}

pub(super) use descriptor::{CheckedRoot, checked_root};

const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;

pub(super) fn checked_relative(value: &str) -> Result<PathBuf, ClosureError> {
    let path = Path::new(value);
    if path.as_os_str().is_empty()
        || path.as_os_str().len() > 4096
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ClosureError::InvalidPath);
    }
    Ok(path.to_path_buf())
}

pub(super) fn slash_path(path: &Path) -> Result<String, ClosureError> {
    let value = path.to_str().ok_or(ClosureError::InvalidPath)?;
    if value.contains('\\') {
        return Err(ClosureError::InvalidPath);
    }
    Ok(value.to_owned())
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct FileIdentity {
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(not(unix))]
    canonical: PathBuf,
}

pub(super) fn hash_stable(
    root: &CheckedRoot,
    relative: &Path,
) -> Result<(String, u64, FileIdentity), ClosureError> {
    let mut file = descriptor::open_confined(root, relative)?;
    let opened = file.metadata().map_err(|_| ClosureError::Unreadable)?;
    validate_regular(&opened, MAX_INPUT_BYTES)?;
    let mut hasher = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| ClosureError::Unreadable)?;
        if read == 0 {
            break;
        }
        bytes = bytes
            .checked_add(read as u64)
            .ok_or(ClosureError::ResourceLimit)?;
        if bytes > MAX_INPUT_BYTES {
            return Err(ClosureError::ResourceLimit);
        }
        hasher.update(&buffer[..read]);
    }
    let after_handle = file.metadata().map_err(|_| ClosureError::Unreadable)?;
    let after_path = descriptor::open_confined(root, relative)
        .map_err(|_| ClosureError::FinalSessionDrift)?
        .metadata()
        .map_err(|_| ClosureError::FinalSessionDrift)?;
    let file_identity = identity(relative, &opened)?;
    if metadata_tuple(&opened) != metadata_tuple(&after_handle)
        || metadata_tuple(&opened) != metadata_tuple(&after_path)
        || opened.len() != bytes
        || after_handle.len() != bytes
        || after_path.len() != bytes
    {
        return Err(ClosureError::FinalSessionDrift);
    }
    Ok((
        format!("sha256:{:x}", hasher.finalize()),
        bytes,
        file_identity,
    ))
}

pub(super) fn read_stable(
    root: &CheckedRoot,
    relative: &Path,
    maximum: u64,
) -> Result<String, ClosureError> {
    let mut file = descriptor::open_confined(root, relative)?;
    let before = file.metadata().map_err(|_| ClosureError::Unreadable)?;
    validate_regular(&before, maximum)?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| ClosureError::Unreadable)?;
    let after = file.metadata().map_err(|_| ClosureError::Unreadable)?;
    let after_path = descriptor::open_confined(root, relative)
        .map_err(|_| ClosureError::FinalSessionDrift)?
        .metadata()
        .map_err(|_| ClosureError::FinalSessionDrift)?;
    if bytes.len() as u64 > maximum
        || bytes.len() as u64 != before.len()
        || metadata_tuple(&before) != metadata_tuple(&after)
        || metadata_tuple(&before) != metadata_tuple(&after_path)
    {
        return Err(ClosureError::FinalSessionDrift);
    }
    String::from_utf8(bytes).map_err(|_| ClosureError::Unreadable)
}

fn validate_regular(metadata: &Metadata, maximum: u64) -> Result<(), ClosureError> {
    if !metadata.is_file() || metadata.len() > maximum || hard_link_count(metadata) != 1 {
        return Err(ClosureError::SpecialFile);
    }
    Ok(())
}

fn identity(path: &Path, metadata: &Metadata) -> Result<FileIdentity, ClosureError> {
    #[cfg(unix)]
    {
        let _ = path;
        Ok(FileIdentity {
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }
    #[cfg(not(unix))]
    {
        Ok(FileIdentity {
            canonical: fs::canonicalize(path).map_err(|_| ClosureError::Unreadable)?,
        })
    }
}

#[cfg(unix)]
fn metadata_tuple(metadata: &Metadata) -> (u64, u64, u32, u64, u64, i64, i64, i64, i64) {
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
fn metadata_tuple(metadata: &Metadata) -> (u64, bool) {
    (metadata.len(), metadata.permissions().readonly())
}

#[cfg(unix)]
fn hard_link_count(metadata: &Metadata) -> u64 {
    metadata.nlink()
}

#[cfg(not(unix))]
fn hard_link_count(_: &Metadata) -> u64 {
    1
}

#[cfg(all(test, unix))]
#[path = "source_closure_filesystem_tests.rs"]
mod tests;
