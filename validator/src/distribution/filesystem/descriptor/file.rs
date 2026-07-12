use super::directory::Directory;
use super::types::{EntryKind, component, file_identity, joined, last_errno, open_error};
use super::{FileIdentity, FileSnapshot};
use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::filesystem::hooks::{self, EffectPoint};
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::MetadataExt;

pub(crate) fn read_file(
    parent: &Directory,
    name: &str,
    maximum: usize,
) -> Result<Option<FileSnapshot>, DistributionError> {
    let name = component(name)?;
    let text = name
        .to_str()
        .map_err(|_| error(DistributionErrorId::InvalidPath))?;
    hooks::before(EffectPoint::OpenFile, &joined(parent.relative(), text));
    let directory = parent.mutation_descriptor()?;
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        )
    };
    if descriptor < 0 {
        return match last_errno() {
            Some(libc::ENOENT) => Ok(None),
            _ => Err(open_error()),
        };
    }
    let mut file = unsafe { File::from_raw_fd(descriptor) };
    let before = file
        .metadata()
        .map_err(|_| error(DistributionErrorId::UnsafeObject))?;
    if !before.is_file() || before.nlink() != 1 || before.dev() != parent.root_device() {
        return Err(error(DistributionErrorId::UnsafeObject));
    }
    if before.len() > maximum as u64 {
        return Err(error(DistributionErrorId::ObjectTooLarge));
    }
    let mut bytes = Vec::with_capacity(before.len() as usize);
    std::io::Read::by_ref(&mut file)
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
    let after = file
        .metadata()
        .map_err(|_| error(DistributionErrorId::ObjectChanged))?;
    let identity = file_identity(&before);
    if bytes.len() > maximum
        || file_identity(&after) != identity
        || after.len() != bytes.len() as u64
    {
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    if !parent.stat(text)?.is_some_and(|row| {
        row.kind == EntryKind::Regular
            && row.links == 1
            && row.identity.device == identity.device
            && row.identity.inode == identity.inode
            && row.length == identity.length
    }) {
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    Ok(Some(FileSnapshot {
        bytes,
        mode: before.mode() & 0o777,
        identity,
    }))
}

pub(crate) fn create_file(
    parent: &Directory,
    name: &str,
    bytes: &[u8],
    mode: u32,
) -> Result<(File, FileIdentity), DistributionError> {
    let name = component(name)?;
    let text = name
        .to_str()
        .map_err(|_| error(DistributionErrorId::InvalidPath))?;
    hooks::before(EffectPoint::CreateFile, &joined(parent.relative(), text));
    let directory = parent.mutation_descriptor()?;
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            0o600,
        )
    };
    if descriptor < 0 {
        return Err(error(DistributionErrorId::EffectFailed));
    }
    let mut file = unsafe { File::from_raw_fd(descriptor) };
    let metadata = file
        .metadata()
        .map_err(|_| error(DistributionErrorId::EffectFailed))?;
    if !metadata.is_file() || metadata.nlink() != 1 || metadata.dev() != parent.root_device() {
        return Err(error(DistributionErrorId::UnsafeObject));
    }
    if unsafe { libc::fchmod(file.as_raw_fd(), mode as libc::mode_t) } != 0 {
        return Err(error(DistributionErrorId::EffectFailed));
    }
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| error(DistributionErrorId::EffectFailed))?;
    let metadata = file
        .metadata()
        .map_err(|_| error(DistributionErrorId::EffectFailed))?;
    Ok((file, file_identity(&metadata)))
}

pub(crate) fn entry_matches_file(
    parent: &Directory,
    name: &str,
    file: &File,
    identity: FileIdentity,
) -> Result<bool, DistributionError> {
    let current = parent.stat(name)?;
    let descriptor = file
        .metadata()
        .map_err(|_| error(DistributionErrorId::ObjectChanged))?;
    Ok(file_identity(&descriptor) == identity
        && current.is_some_and(|row| {
            row.kind == EntryKind::Regular
                && row.links == 1
                && row.identity.device == identity.device
                && row.identity.inode == identity.inode
                && row.length == identity.length
        }))
}

pub(crate) fn entry_matches_file_after_rename(
    parent: &Directory,
    name: &str,
    file: &File,
    identity: FileIdentity,
) -> Result<bool, DistributionError> {
    let current = parent.stat(name)?;
    let descriptor = file
        .metadata()
        .map_err(|_| error(DistributionErrorId::ObjectChanged))?;
    Ok(file_identity(&descriptor).same_after_rename(identity)
        && current.is_some_and(|row| {
            row.kind == EntryKind::Regular
                && row.links == 1
                && row.identity.device == identity.device
                && row.identity.inode == identity.inode
                && row.length == identity.length
        }))
}
