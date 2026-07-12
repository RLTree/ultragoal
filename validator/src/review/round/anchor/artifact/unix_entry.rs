use super::{
    MAX_ANCHOR_BYTES, NOT_REGULAR, PATH_INVALID, TOO_LARGE, UNAVAILABLE, initial_open_error,
};
use std::{
    ffi::{CString, OsStr},
    fs::File,
    mem::MaybeUninit,
    os::{
        fd::AsRawFd,
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::Path,
};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) struct Snapshot {
    device: u64,
    inode: u64,
    links: u64,
    pub(super) length: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}

pub(super) fn directory_identity(file: &File) -> Result<(u64, u64), String> {
    let metadata = file.metadata().map_err(|_| PATH_INVALID)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(PATH_INVALID.to_string());
    }
    Ok((metadata.dev(), metadata.ino()))
}

pub(super) fn path_directory_identity(path: &Path) -> Result<(u64, u64), String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| PATH_INVALID)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(PATH_INVALID.to_string());
    }
    Ok((metadata.dev(), metadata.ino()))
}

pub(super) fn directory_identity_at(parent: &File, name: &OsStr) -> Result<(u64, u64), String> {
    let metadata = stat_at(parent, name).map_err(initial_open_error)?;
    if file_type(metadata.st_mode) != libc::S_IFDIR {
        return Err(PATH_INVALID.to_string());
    }
    Ok((metadata.st_dev as u64, metadata.st_ino as u64))
}

pub(super) fn initial_path_snapshot(parent: &File, name: &OsStr) -> Result<Snapshot, String> {
    let metadata = stat_at(parent, name).map_err(initial_open_error)?;
    initial_stat_snapshot(&metadata)
}

pub(super) fn initial_file_snapshot(file: &File) -> Result<Snapshot, String> {
    let metadata = file.metadata().map_err(|_| UNAVAILABLE)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.nlink() != 1 {
        return Err(NOT_REGULAR.to_string());
    }
    if metadata.len() > MAX_ANCHOR_BYTES {
        return Err(TOO_LARGE.to_string());
    }
    file_snapshot(file)
}

pub(super) fn file_snapshot(file: &File) -> Result<Snapshot, String> {
    let metadata = file.metadata().map_err(|_| UNAVAILABLE)?;
    Ok(Snapshot {
        device: metadata.dev(),
        inode: metadata.ino(),
        links: metadata.nlink(),
        length: metadata.len(),
        modified: (metadata.mtime(), metadata.mtime_nsec()),
        changed: (metadata.ctime(), metadata.ctime_nsec()),
    })
}

fn initial_stat_snapshot(metadata: &libc::stat) -> Result<Snapshot, String> {
    let kind = file_type(metadata.st_mode);
    if kind == libc::S_IFLNK {
        return Err(PATH_INVALID.to_string());
    }
    if kind != libc::S_IFREG || metadata.st_nlink != 1 {
        return Err(NOT_REGULAR.to_string());
    }
    let length = u64::try_from(metadata.st_size).map_err(|_| UNAVAILABLE.to_string())?;
    if length > MAX_ANCHOR_BYTES {
        return Err(TOO_LARGE.to_string());
    }
    Ok(Snapshot {
        device: metadata.st_dev as u64,
        inode: metadata.st_ino as u64,
        links: metadata.st_nlink as u64,
        length,
        modified: (metadata.st_mtime as i64, metadata.st_mtime_nsec as i64),
        changed: (metadata.st_ctime as i64, metadata.st_ctime_nsec as i64),
    })
}

fn stat_at(parent: &File, name: &OsStr) -> std::io::Result<libc::stat> {
    let name = CString::new(name.as_bytes())?;
    let mut metadata = MaybeUninit::<libc::stat>::uninit();
    let status = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            metadata.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if status != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { metadata.assume_init() })
}

fn file_type(mode: libc::mode_t) -> libc::mode_t {
    mode & libc::S_IFMT
}
