use super::*;
use std::ffi::CString;
use std::fs::OpenOptions;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::OpenOptionsExt;

pub(super) fn create_exclusive_at(
    directory: &File,
    name: &str,
    mode: u32,
) -> std::io::Result<File> {
    let name = CString::new(name)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid name"))?;
    // SAFETY: directory is live, name is one NUL-terminated component, and a
    // successful openat returns one newly owned descriptor.
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            mode,
        )
    };
    if descriptor < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        // SAFETY: the successful descriptor is newly owned.
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }
}

pub(super) fn create_bound_directory_at(_root: &Path, _name: &str) -> std::io::Result<File> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "identity-bound directory creation is unavailable on this platform",
    ))
}

pub(super) fn validate_directory_path(
    child: &Path,
    expected: ObjectIdentity,
) -> Result<(), RoutineError> {
    let metadata = fs::symlink_metadata(child)
        .map_err(|_| error("routine-production-launch-directory-replaced"))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || ObjectIdentity::from(&metadata) != expected
    {
        return Err(error("routine-production-launch-directory-replaced"));
    }
    let directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(child)
        .map_err(|_| error("routine-production-launch-directory-replaced"))?;
    if ObjectIdentity::from(
        &directory
            .metadata()
            .map_err(|_| error("routine-production-launch-directory-replaced"))?,
    ) != expected
    {
        return Err(error("routine-production-launch-directory-replaced"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "snapshot_bound_filesystem_tests.rs"]
mod tests;
