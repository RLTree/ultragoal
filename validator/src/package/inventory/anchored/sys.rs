use super::snapshot::Snapshot;
use std::ffi::{CString, OsStr, OsString};
use std::fs::{File, OpenOptions};
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::path::{Component, Path};

pub(super) const MAX_PATH_BYTES: usize = 4096;
pub(super) const MAX_COMPONENTS: usize = 128;

pub(super) fn components(relative: &str) -> Result<Vec<OsString>, String> {
    if relative.is_empty() || relative.len() > MAX_PATH_BYTES {
        return Err("anchored package path exceeds its byte limit".to_string());
    }
    let values = Path::new(relative)
        .components()
        .map(|component| match component {
            Component::Normal(name) if !name.as_bytes().is_empty() => Ok(name.to_os_string()),
            _ => Err("anchored package path is not confined".to_string()),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if values.is_empty() || values.len() > MAX_COMPONENTS {
        return Err("anchored package path exceeds its component limit".to_string());
    }
    Ok(values)
}

pub(super) fn open_root(root: &Path) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
    options
        .open(root)
        .map_err(|_| "anchored package root open failed".to_string())
}

pub(super) fn open_directory(parent: &File, name: &OsStr) -> Result<File, String> {
    open_at(
        parent,
        name,
        libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
    )
}

pub(super) fn open_file(parent: &File, name: &OsStr) -> Result<File, String> {
    open_at(
        parent,
        name,
        libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
    )
}

fn open_at(parent: &File, name: &OsStr, flags: i32) -> Result<File, String> {
    let name = CString::new(name.as_bytes())
        .map_err(|_| "anchored package path contains NUL".to_string())?;
    let descriptor = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        return Err("anchored package component open failed".to_string());
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

pub(super) fn nofollow_snapshot(parent: &File, name: &OsStr) -> Result<Snapshot, String> {
    let name = CString::new(name.as_bytes())
        .map_err(|_| "anchored package path contains NUL".to_string())?;
    let mut stat = MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        return Err("anchored package component metadata failed".to_string());
    }
    let stat = unsafe { stat.assume_init() };
    Ok(Snapshot::from_stat(&stat))
}
