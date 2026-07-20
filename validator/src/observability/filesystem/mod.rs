use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

#[cfg(test)]
use std::cell::Cell;

#[cfg(test)]
thread_local! {
    static FAIL_CREATED_LEAF_VALIDATION: Cell<bool> = const { Cell::new(false) };
}

#[cfg(test)]
pub(crate) fn fail_next_created_leaf_validation() {
    FAIL_CREATED_LEAF_VALIDATION.with(|fail| fail.set(true));
}

mod anchors;
mod confinement;

pub(super) use confinement::VerifiedParent;

use confinement::io_code;
#[cfg(not(unix))]
use confinement::unsupported_platform;
#[cfg(unix)]
use confinement::{
    created_leaf_recovery_required, identity, inspect_leaf, open_leaf, open_verified_parent,
    validate_file, validate_metadata,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct FileIdentity {
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

pub(super) fn bind_parent(path: &Path) -> Result<VerifiedParent, String> {
    #[cfg(not(unix))]
    {
        let _ = path;
        return Err(unsupported_platform());
    }
    #[cfg(unix)]
    {
        open_verified_parent(path)
    }
}

pub(super) fn identity_if_exists(parent: &VerifiedParent) -> Result<Option<FileIdentity>, String> {
    #[cfg(not(unix))]
    {
        let _ = parent;
        return Err(unsupported_platform());
    }
    #[cfg(unix)]
    {
        parent.revalidate()?;
        inspect_leaf(parent)
    }
}

pub(super) fn open_read(
    parent: &VerifiedParent,
    expected: Option<FileIdentity>,
) -> Result<Option<File>, String> {
    #[cfg(not(unix))]
    {
        let _ = (parent, expected);
        return Err(unsupported_platform());
    }
    #[cfg(unix)]
    {
        parent.revalidate()?;
        if inspect_leaf(parent)?.is_none() {
            return Ok(None);
        }
        let file = open_leaf(parent, libc::O_RDONLY, "open-read")?;
        validate_file(parent, &file, expected)?;
        Ok(Some(file))
    }
}

pub(super) fn open_append(
    parent: &VerifiedParent,
    expected: Option<FileIdentity>,
) -> Result<(File, bool), String> {
    #[cfg(not(unix))]
    {
        let _ = (parent, expected);
        return Err(unsupported_platform());
    }
    #[cfg(unix)]
    {
        parent.revalidate()?;
        if expected.is_none() {
            match open_leaf(
                parent,
                libc::O_RDWR | libc::O_APPEND | libc::O_CREAT | libc::O_EXCL,
                "open-append",
            ) {
                Ok(file) => {
                    let created = file_identity(&file)?;
                    if !validate_created_leaf(parent, &file) {
                        return Err(created_leaf_recovery_required(parent, created));
                    }
                    return Ok((file, true));
                }
                Err(error) if error == "observe-store-open-append:already-exists" => {}
                Err(error) => return Err(error),
            }
        }
        inspect_leaf(parent)?.ok_or_else(|| "observe-store-open-append:not-found".to_owned())?;
        let file = open_leaf(parent, libc::O_RDWR | libc::O_APPEND, "open-append")?;
        validate_file(parent, &file, expected)?;
        Ok((file, false))
    }
}

#[cfg(unix)]
fn validate_created_leaf(parent: &VerifiedParent, file: &File) -> bool {
    #[cfg(test)]
    if FAIL_CREATED_LEAF_VALIDATION.with(|fail| fail.replace(false)) {
        return false;
    }
    validate_file(parent, file, None).is_ok()
}

pub(super) fn open_write_existing(
    parent: &VerifiedParent,
    expected: Option<FileIdentity>,
) -> Result<Option<File>, String> {
    #[cfg(not(unix))]
    {
        let _ = (parent, expected);
        return Err(unsupported_platform());
    }
    #[cfg(unix)]
    {
        parent.revalidate()?;
        if inspect_leaf(parent)?.is_none() {
            return Ok(None);
        }
        let file = open_leaf(parent, libc::O_RDWR, "open-write")?;
        validate_file(parent, &file, expected)?;
        Ok(Some(file))
    }
}

pub(super) fn read_bounded(file: &mut File, max_bytes: u64) -> Result<Vec<u8>, String> {
    let length = file
        .metadata()
        .map_err(|error| io_code("metadata", error))?
        .len();
    if length > max_bytes {
        return Err("observe-store-limit: byte bound exceeded".to_owned());
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|error| io_code("seek", error))?;
    let mut bytes = Vec::with_capacity(length as usize);
    file.take(max_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| io_code("read", error))?;
    if bytes.len() as u64 > max_bytes {
        return Err("observe-store-limit: byte bound exceeded".to_owned());
    }
    Ok(bytes)
}

pub(super) fn validate_bound_parent(
    parent: &VerifiedParent,
    file: &File,
    expected: Option<FileIdentity>,
) -> Result<(), String> {
    #[cfg(not(unix))]
    {
        let _ = (parent, file, expected);
        return Err(unsupported_platform());
    }
    #[cfg(unix)]
    {
        validate_file(parent, file, expected)
    }
}

pub(super) fn file_identity(file: &File) -> Result<FileIdentity, String> {
    #[cfg(not(unix))]
    {
        let _ = file;
        return Err(unsupported_platform());
    }
    #[cfg(unix)]
    {
        let metadata = file
            .metadata()
            .map_err(|error| io_code("metadata", error))?;
        validate_metadata(&metadata)?;
        Ok(identity(&metadata))
    }
}
