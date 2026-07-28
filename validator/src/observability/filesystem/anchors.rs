use std::ffi::CStr;
use std::fs::File;
use std::mem::MaybeUninit;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

use super::FileIdentity;
use super::confinement::io_code;

pub(super) fn absolute_parent(
    parent: &Path,
    cwd_identity: FileIdentity,
) -> Result<(PathBuf, usize, FileIdentity), String> {
    let cwd = std::env::current_dir()
        .map_err(|_| "observe-store-path-denied: current directory unavailable".to_owned())?;
    if !cwd.is_absolute() {
        return Err("observe-store-path-denied: current directory unavailable".to_owned());
    }
    let depth = cwd
        .components()
        .filter(|component| matches!(component, Component::Normal(_)))
        .count();
    Ok((cwd.join(parent), depth, cwd_identity))
}

pub(super) fn validate_directory_link(
    parent: libc::c_int,
    name: &CStr,
    expected: FileIdentity,
) -> Result<(), String> {
    let mut stat = MaybeUninit::<libc::stat>::uninit();
    // SAFETY: `parent` is a caller-held directory descriptor, `name` is a
    // NUL-terminated C string, and `stat` is valid writable storage.
    let result = unsafe {
        libc::fstatat(
            parent,
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        return Err("observe-store-path-denied: ancestor substitution detected".to_owned());
    }
    // SAFETY: successful `fstatat` initialized the complete `libc::stat`.
    let stat = unsafe { stat.assume_init() };
    if (stat.st_mode & libc::S_IFMT) == libc::S_IFLNK {
        return Err("observe-store-path-denied: ancestor symlink rejected".to_owned());
    }
    if (stat.st_mode & libc::S_IFMT) != libc::S_IFDIR
        || (FileIdentity {
            device: stat_device(stat.st_dev)?,
            inode: stat.st_ino,
        }) != expected
    {
        return Err("observe-store-path-denied: ancestor substitution detected".to_owned());
    }
    Ok(())
}

pub(super) fn directory_identity(directory: &File) -> Result<FileIdentity, String> {
    let metadata = directory
        .metadata()
        .map_err(|error| io_code("metadata", error))?;
    if !metadata.is_dir() {
        return Err("observe-store-path-denied: parent is not a directory".to_owned());
    }
    Ok(FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

pub(super) fn validate_stat(stat: &libc::stat) -> Result<(), String> {
    match stat.st_mode & libc::S_IFMT {
        libc::S_IFLNK => Err("observe-store-path-denied: symlink store rejected".to_owned()),
        libc::S_IFREG if stat.st_nlink == 1 => Ok(()),
        libc::S_IFREG => Err("observe-store-path-denied: hardlinked store rejected".to_owned()),
        _ => Err("observe-store-path-denied: special store rejected".to_owned()),
    }
}

pub(super) fn identity_stat(stat: &libc::stat) -> Result<FileIdentity, String> {
    Ok(FileIdentity {
        device: stat_device(stat.st_dev)?,
        inode: stat.st_ino,
    })
}

fn stat_device(value: libc::dev_t) -> Result<u64, String> {
    u64::try_from(value)
        .map_err(|_| "observe-store-path-denied: filesystem identity is invalid".to_owned())
}
