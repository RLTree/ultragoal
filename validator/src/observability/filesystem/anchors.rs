use std::ffi::CStr;
use std::fs::File;
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
    let mut stat = unsafe { std::mem::zeroed::<libc::stat>() };
    let result =
        unsafe { libc::fstatat(parent, name.as_ptr(), &mut stat, libc::AT_SYMLINK_NOFOLLOW) };
    if result != 0 {
        return Err("observe-store-path-denied: ancestor substitution detected".to_owned());
    }
    if (stat.st_mode & libc::S_IFMT) == libc::S_IFLNK {
        return Err("observe-store-path-denied: ancestor symlink rejected".to_owned());
    }
    if (stat.st_mode & libc::S_IFMT) != libc::S_IFDIR
        || (FileIdentity {
            device: stat.st_dev as u64,
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

pub(super) fn identity_stat(stat: &libc::stat) -> FileIdentity {
    FileIdentity {
        device: stat.st_dev as u64,
        inode: stat.st_ino,
    }
}
