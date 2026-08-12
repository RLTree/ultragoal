use crate::context::error::{ContextError, io_error};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

#[cfg(unix)]
use std::ffi::{CString, OsStr};
#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

const MAX_FILE_BYTES: u64 = 128 * 1024 * 1024;

#[cfg(unix)]
fn same_regular_snapshot(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.is_file()
        && right.is_file()
        && left.dev() == right.dev()
        && left.ino() == right.ino()
        && left.mode() == right.mode()
        && left.nlink() == right.nlink()
        && left.len() == right.len()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
        && left.ctime() == right.ctime()
        && left.ctime_nsec() == right.ctime_nsec()
}

#[cfg(unix)]
fn openat_file(parent: &File, name: &OsStr, flags: i32, path: &Path) -> Result<File, ContextError> {
    let name = CString::new(name.as_bytes()).map_err(|_| {
        ContextError::PathDenied(format!(
            "untracked path contains a NUL component: {}",
            path.display()
        ))
    })?;
    // SAFETY: `parent` is an owned live descriptor, `name` is a live
    // NUL-terminated component, and the fixed flags do not create a file.
    let descriptor = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        return Err(io_error(path, std::io::Error::last_os_error()));
    }
    // SAFETY: a successful `openat` returned one newly owned descriptor.
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

#[cfg(unix)]
fn open_regular(root: &Path, relative: &Path, before: &fs::Metadata) -> Result<File, ContextError> {
    if !before.is_file()
        || before.file_type().is_symlink()
        || before.nlink() != 1
        || before.len() > MAX_FILE_BYTES
    {
        return Err(ContextError::PathDenied(format!(
            "untracked regular file has unsafe type, links, or size: {}",
            root.join(relative).display()
        )));
    }
    let root_before = fs::symlink_metadata(root).map_err(|error| io_error(root, error))?;
    if !root_before.is_dir() || root_before.file_type().is_symlink() {
        return Err(ContextError::ConcurrentMutation(
            "untracked capture worktree root identity".to_owned(),
        ));
    }
    let mut components = relative.components().collect::<Vec<_>>();
    let leaf = components.pop().ok_or_else(|| {
        ContextError::PathDenied("untracked path has no final component".to_owned())
    })?;
    if !matches!(leaf, std::path::Component::Normal(_))
        || components
            .iter()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(ContextError::PathDenied(format!(
            "untracked descriptor path contains a non-normal component: {}",
            relative.display()
        )));
    }
    let mut options = fs::OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK);
    let mut directory = options.open(root).map_err(|error| io_error(root, error))?;
    let opened_root = directory
        .metadata()
        .map_err(|error| io_error(root, error))?;
    if !opened_root.is_dir()
        || root_before.dev() != opened_root.dev()
        || root_before.ino() != opened_root.ino()
    {
        return Err(ContextError::ConcurrentMutation(
            "untracked capture worktree root identity".to_owned(),
        ));
    }
    let target = root.join(relative);
    for component in components {
        directory = openat_file(
            &directory,
            component.as_os_str(),
            libc::O_RDONLY
                | libc::O_DIRECTORY
                | libc::O_NOFOLLOW
                | libc::O_CLOEXEC
                | libc::O_NONBLOCK,
            &target,
        )?;
        if !directory
            .metadata()
            .map_err(|error| io_error(&target, error))?
            .is_dir()
        {
            return Err(ContextError::PathDenied(format!(
                "untracked ancestor is not a directory: {}",
                target.display()
            )));
        }
    }
    let file = openat_file(
        &directory,
        leaf.as_os_str(),
        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        &target,
    )?;
    let opened = file.metadata().map_err(|error| io_error(&target, error))?;
    if !same_regular_snapshot(before, &opened) {
        return Err(ContextError::ConcurrentMutation(format!(
            "untracked file changed during descriptor binding: {}",
            target.display()
        )));
    }
    Ok(file)
}

#[cfg(not(unix))]
fn open_regular(
    root: &Path,
    relative: &Path,
    _before: &fs::Metadata,
) -> Result<File, ContextError> {
    Err(ContextError::UnsupportedCapability(format!(
        "secure untracked-file capture requires no-follow descriptor identity: {}",
        root.join(relative).display()
    )))
}

pub(super) fn hash(
    hasher: &mut Sha256,
    root: &Path,
    relative: &Path,
    before: &fs::Metadata,
) -> Result<(), ContextError> {
    let path = root.join(relative);
    let mut file = open_regular(root, relative, before)?;
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| io_error(&path, error))?;
        if read == 0 {
            break;
        }
        bytes = bytes.checked_add(read as u64).ok_or_else(|| {
            ContextError::PathDenied(format!("untracked file size overflow: {}", path.display()))
        })?;
        if bytes > MAX_FILE_BYTES {
            return Err(ContextError::PathDenied(format!(
                "untracked file exceeds capture byte limit: {}",
                path.display()
            )));
        }
        hasher.update(&buffer[..read]);
    }
    if bytes != before.len() {
        return Err(ContextError::ConcurrentMutation(format!(
            "untracked file size changed during capture: {}",
            path.display()
        )));
    }
    #[cfg(unix)]
    {
        let opened_after = file.metadata().map_err(|error| io_error(&path, error))?;
        let path_after = fs::symlink_metadata(&path).map_err(|error| io_error(&path, error))?;
        if !same_regular_snapshot(before, &opened_after)
            || !same_regular_snapshot(&opened_after, &path_after)
        {
            return Err(ContextError::ConcurrentMutation(format!(
                "untracked file identity changed during capture: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

#[cfg(all(test, unix))]
pub(in crate::context) fn open_regular_for_test(
    root: &Path,
    relative: &Path,
    before: &fs::Metadata,
) -> Result<File, ContextError> {
    open_regular(root, relative, before)
}
