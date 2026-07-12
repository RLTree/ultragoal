use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use std::ffi::CString;
use std::os::unix::fs::MetadataExt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DirectoryIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EntryKind {
    Directory,
    Regular,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct EntryMetadata {
    pub(crate) identity: DirectoryIdentity,
    pub(crate) kind: EntryKind,
    pub(crate) links: u64,
    pub(crate) length: u64,
}

pub(crate) struct FileSnapshot {
    pub(crate) bytes: Vec<u8>,
    pub(crate) mode: u32,
    pub(crate) identity: FileIdentity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FileIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) length: u64,
    pub(crate) modified_seconds: i64,
    pub(crate) modified_nanos: i64,
    pub(crate) changed_seconds: i64,
    pub(crate) changed_nanos: i64,
}

impl FileIdentity {
    // A rename changes ctime on supported hosts even though the same file and
    // bytes moved. Callers using this comparison also bind the parent entry,
    // link count, mode, and bytes; ctime remains part of strict pre-effect
    // equality.
    pub(crate) fn same_after_rename(self, other: Self) -> bool {
        self.device == other.device
            && self.inode == other.inode
            && self.length == other.length
            && self.modified_seconds == other.modified_seconds
            && self.modified_nanos == other.modified_nanos
    }
}

pub(crate) fn component(value: &str) -> Result<CString, DistributionError> {
    if value.is_empty()
        || value.len() > 255
        || value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
        || !value.is_ascii()
        || value
            .bytes()
            .any(|byte| !(byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')))
    {
        return Err(error(DistributionErrorId::InvalidPath));
    }
    CString::new(value).map_err(|_| error(DistributionErrorId::InvalidPath))
}

pub(crate) fn directory_identity(metadata: &std::fs::Metadata) -> DirectoryIdentity {
    DirectoryIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    }
}

pub(crate) fn file_identity(metadata: &std::fs::Metadata) -> FileIdentity {
    FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        length: metadata.len(),
        modified_seconds: metadata.mtime(),
        modified_nanos: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanos: metadata.ctime_nsec(),
    }
}

pub(crate) fn joined(parent: &str, child: &str) -> String {
    if parent.is_empty() {
        child.to_owned()
    } else {
        format!("{parent}/{child}")
    }
}

pub(crate) fn last_errno() -> Option<i32> {
    std::io::Error::last_os_error().raw_os_error()
}

pub(crate) fn open_error() -> DistributionError {
    match last_errno() {
        Some(libc::ENOENT) => error(DistributionErrorId::ObjectUnavailable),
        Some(libc::EFBIG) => error(DistributionErrorId::ObjectTooLarge),
        _ => error(DistributionErrorId::UnsafeObject),
    }
}
