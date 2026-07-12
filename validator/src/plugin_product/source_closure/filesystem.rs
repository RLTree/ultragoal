use super::model::ClosureError;
use sha2::{Digest, Sha256};
use std::fs::{self, File, Metadata};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;

pub(super) fn checked_root(root: &Path) -> Result<PathBuf, ClosureError> {
    let metadata = fs::symlink_metadata(root).map_err(|_| ClosureError::Missing)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ClosureError::SpecialFile);
    }
    fs::canonicalize(root).map_err(|_| ClosureError::Unreadable)
}

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

pub(super) fn checked_file(root: &Path, relative: &Path) -> Result<PathBuf, ClosureError> {
    let mut current = root.to_path_buf();
    let components = relative.components().collect::<Vec<_>>();
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(component) = component else {
            return Err(ClosureError::InvalidPath);
        };
        current.push(component);
        let metadata = fs::symlink_metadata(&current).map_err(|_| ClosureError::Missing)?;
        if metadata.file_type().is_symlink() || (index + 1 < components.len() && !metadata.is_dir())
        {
            return Err(ClosureError::SpecialFile);
        }
    }
    let metadata = fs::symlink_metadata(&current).map_err(|_| ClosureError::Missing)?;
    if !metadata.is_file() || metadata.len() > MAX_INPUT_BYTES || hard_link_count(&metadata) != 1 {
        return Err(ClosureError::SpecialFile);
    }
    let canonical = fs::canonicalize(&current).map_err(|_| ClosureError::Unreadable)?;
    if !canonical.starts_with(root) || canonical != current {
        return Err(ClosureError::OutsideRoot);
    }
    Ok(current)
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

pub(super) fn hash_stable(path: &Path) -> Result<(String, u64, FileIdentity), ClosureError> {
    let before = fs::symlink_metadata(path).map_err(|_| ClosureError::Missing)?;
    let mut file = File::open(path).map_err(|_| ClosureError::Unreadable)?;
    let opened = file.metadata().map_err(|_| ClosureError::Unreadable)?;
    if identity(path, &before)? != identity(path, &opened)? {
        return Err(ClosureError::FinalSessionDrift);
    }
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
    let after_path = fs::symlink_metadata(path).map_err(|_| ClosureError::FinalSessionDrift)?;
    let file_identity = identity(path, &opened)?;
    if file_identity != identity(path, &after_handle)?
        || file_identity != identity(path, &after_path)?
        || before.len() != bytes
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
fn hard_link_count(metadata: &Metadata) -> u64 {
    metadata.nlink()
}

#[cfg(not(unix))]
fn hard_link_count(_: &Metadata) -> u64 {
    1
}
