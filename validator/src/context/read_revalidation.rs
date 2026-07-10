use super::error::{ContextError, io_error};
use super::read_snapshot::{FileSnapshot, snapshot};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

fn concurrent(path: &Path) -> ContextError {
    ContextError::ConcurrentMutation(format!("read-session observation: {}", path.display()))
}

pub(super) fn regular(
    path: &Path,
    expected_digest: Option<&str>,
    expected_length: u64,
    expected_identity: &FileSnapshot,
    descriptor: &File,
) -> Result<(), ContextError> {
    let path_before = fs::symlink_metadata(path).map_err(|error| io_error(path, error))?;
    let mut file = descriptor
        .try_clone()
        .map_err(|error| io_error(path, error))?;
    let opened_before = file.metadata().map_err(|error| io_error(path, error))?;
    if snapshot(&path_before) != *expected_identity
        || snapshot(&opened_before) != *expected_identity
    {
        return Err(concurrent(path));
    }
    let content = expected_digest
        .map(|expected| {
            read_content(path, &mut file, expected_length).map(|actual| (expected, actual))
        })
        .transpose()?;
    let opened_after = file.metadata().map_err(|error| io_error(path, error))?;
    let path_after = fs::symlink_metadata(path).map_err(|error| io_error(path, error))?;
    if snapshot(&opened_after) != *expected_identity
        || snapshot(&path_after) != *expected_identity
        || content.is_some_and(|(expected, actual)| expected != actual)
    {
        return Err(concurrent(path));
    }
    Ok(())
}

fn read_content(
    path: &Path,
    file: &mut File,
    expected_length: u64,
) -> Result<String, ContextError> {
    file.seek(SeekFrom::Start(0))
        .map_err(|error| io_error(path, error))?;
    let mut hasher = Sha256::new();
    let mut length = 0_u64;
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| io_error(path, error))?;
        if read == 0 {
            break;
        }
        length = length.saturating_add(read as u64);
        if length > expected_length {
            return Err(concurrent(path));
        }
        hasher.update(&buffer[..read]);
    }
    if length != expected_length {
        return Err(concurrent(path));
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub(super) fn symlink(
    path: &Path,
    expected_digest: &str,
    expected_identity: &FileSnapshot,
) -> Result<(), ContextError> {
    let before = fs::symlink_metadata(path).map_err(|error| io_error(path, error))?;
    let target = fs::read_link(path).map_err(|error| io_error(path, error))?;
    let after = fs::symlink_metadata(path).map_err(|error| io_error(path, error))?;
    if snapshot(&before) != *expected_identity
        || snapshot(&after) != *expected_identity
        || format!("{:x}", Sha256::digest(target.as_os_str().as_bytes())) != expected_digest
    {
        return Err(concurrent(path));
    }
    Ok(())
}
