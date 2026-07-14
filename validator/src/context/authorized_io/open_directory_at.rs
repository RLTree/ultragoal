use super::*;

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(crate) fn open_directory_at(
    directory: &File,
    name: &std::ffi::OsStr,
    target: &Path,
) -> Result<File, ContextError> {
    let name = c_name(name, target)?;
    let flags = O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC;
    let descriptor = unsafe { openat(directory.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        return Err(io_error(target, std::io::Error::last_os_error()));
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(crate) fn open_regular_at(
    directory: &File,
    name: &std::ffi::OsStr,
    target: &Path,
) -> Result<File, ContextError> {
    let name = c_name(name, target)?;
    let flags = O_RDONLY | O_NOFOLLOW | O_CLOEXEC;
    let descriptor = unsafe { openat(directory.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        return Err(io_error(target, std::io::Error::last_os_error()));
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(crate) fn c_name(name: &std::ffi::OsStr, target: &Path) -> Result<CString, ContextError> {
    CString::new(name.as_bytes()).map_err(|_| {
        ContextError::PathDenied(format!(
            "authorized target contains a NUL byte: {}",
            target.display()
        ))
    })
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub(crate) fn open_anchored(
    _worktree: &Path,
    _recorded_worktree_identity: Option<(u64, u64)>,
    _target: &Path,
    _writing: bool,
    _create_new: bool,
) -> Result<File, ContextError> {
    Err(ContextError::PathDenied(
        "descriptor-anchored I/O is supported only on macOS and Linux".to_owned(),
    ))
}

#[cfg(target_os = "macos")]
pub(crate) const O_RDONLY: i32 = 0;
#[cfg(target_os = "macos")]
pub(crate) const O_WRONLY: i32 = 1;
#[cfg(target_os = "macos")]
pub(crate) const O_CREAT: i32 = 0x0200;
#[cfg(target_os = "macos")]
pub(crate) const O_EXCL: i32 = 0x0800;
#[cfg(target_os = "macos")]
pub(crate) const O_NOFOLLOW: i32 = 0x0100;
#[cfg(target_os = "macos")]
pub(crate) const O_CLOEXEC: i32 = 0x01000000;
#[cfg(target_os = "macos")]
pub(crate) const O_DIRECTORY: i32 = 0x00100000;

#[cfg(target_os = "linux")]
pub(crate) const O_RDONLY: i32 = 0;
#[cfg(target_os = "linux")]
pub(crate) const O_WRONLY: i32 = 1;
#[cfg(target_os = "linux")]
pub(crate) const O_CREAT: i32 = 0x40;
#[cfg(target_os = "linux")]
pub(crate) const O_EXCL: i32 = 0x80;
#[cfg(target_os = "linux")]
pub(crate) const O_NOFOLLOW: i32 = 0x20000;
#[cfg(target_os = "linux")]
pub(crate) const O_CLOEXEC: i32 = 0x80000;
#[cfg(target_os = "linux")]
pub(crate) const O_DIRECTORY: i32 = 0x10000;
