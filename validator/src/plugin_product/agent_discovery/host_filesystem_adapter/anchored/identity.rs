use super::identity_syscall_adapter;
use super::{
    AnchoredDirectory, DirectoryFingerprint, DirectoryHandle, FileFingerprint,
    record_metadata_probe,
};
use crate::plugin_product::agent_discovery::error::AgentDiscoveryError;
use crate::plugin_product::agent_discovery::filesystem::{digest, too_large, unsafe_entry};
use std::ffi::{CStr, CString, OsStr};
use std::fs::{File, Metadata};
use std::os::fd::{FromRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::Arc;

pub(super) fn open_directory_path(path: &Path) -> Result<AnchoredDirectory, AgentDiscoveryError> {
    let path = CString::new(path.as_os_str().as_bytes()).map_err(|_| unsafe_entry())?;
    let raw = identity_syscall_adapter::open_root(&path).map_err(|_| unsafe_entry())?;
    let file = unsafe { File::from_raw_fd(raw) };
    anchored_directory(file)
}

pub(super) fn anchored_directory(file: File) -> Result<AnchoredDirectory, AgentDiscoveryError> {
    let fingerprint = directory_fingerprint(&file.metadata().map_err(|_| unsafe_entry())?)?;
    Ok(AnchoredDirectory {
        handle: Arc::new(DirectoryHandle { file }),
        fingerprint,
    })
}

pub(super) fn component(name: &OsStr) -> Result<CString, AgentDiscoveryError> {
    let bytes = name.as_bytes();
    if bytes.is_empty() || bytes == b"." || bytes == b".." || bytes.contains(&b'/') {
        return Err(unsafe_entry());
    }
    CString::new(bytes).map_err(|_| unsafe_entry())
}

pub(super) fn directory_fingerprint(
    metadata: &Metadata,
) -> Result<DirectoryFingerprint, AgentDiscoveryError> {
    if !metadata.is_dir() {
        return Err(unsafe_entry());
    }
    Ok(DirectoryFingerprint {
        modified_seconds: metadata.mtime(),
        modified_nanos: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanos: metadata.ctime_nsec(),
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

pub(super) fn file_fingerprint(
    metadata: &Metadata,
) -> Result<FileFingerprint, AgentDiscoveryError> {
    if !metadata.is_file() {
        return Err(unsafe_entry());
    }
    Ok(FileFingerprint {
        len: metadata.len(),
        modified_seconds: metadata.mtime(),
        modified_nanos: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanos: metadata.ctime_nsec(),
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

pub(super) fn same_directory_identity(
    left: &DirectoryFingerprint,
    right: &DirectoryFingerprint,
) -> bool {
    left.device == right.device && left.inode == right.inode
}

pub(super) fn directory_identity_sha256(
    label: &[u8],
    fingerprint: &DirectoryFingerprint,
) -> String {
    let mut bytes = Vec::with_capacity(label.len() + 56);
    bytes.extend_from_slice(label);
    bytes.extend_from_slice(&fingerprint.modified_seconds.to_le_bytes());
    bytes.extend_from_slice(&fingerprint.modified_nanos.to_le_bytes());
    bytes.extend_from_slice(&fingerprint.changed_seconds.to_le_bytes());
    bytes.extend_from_slice(&fingerprint.changed_nanos.to_le_bytes());
    bytes.extend_from_slice(&fingerprint.device.to_le_bytes());
    bytes.extend_from_slice(&fingerprint.inode.to_le_bytes());
    digest(&bytes)
}

pub(super) fn require_regular_single_link(
    metadata: &Metadata,
    maximum: usize,
) -> Result<(), AgentDiscoveryError> {
    if !metadata.is_file() || metadata.nlink() != 1 {
        return Err(unsafe_entry());
    }
    if metadata.len() > maximum as u64 {
        return Err(too_large());
    }
    Ok(())
}

pub(super) fn require_regular_single_link_at(
    directory_fd: RawFd,
    name: &CStr,
) -> Result<(), AgentDiscoveryError> {
    record_metadata_probe();
    identity_syscall_adapter::require_regular_single_link_at(directory_fd, name)
        .map_err(|_| unsafe_entry())
}
