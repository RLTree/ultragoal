use super::super::{HostFailure, validate_name};
use std::ffi::CString;
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd};

pub(crate) fn openat(
    parent: &File,
    name: &str,
    flags: i32,
    mode: u32,
) -> Result<File, HostFailure> {
    validate_name(name)?;
    let name = CString::new(name).map_err(|_| HostFailure::Invalid)?;
    // SAFETY: the descriptor is borrowed, the C string is NUL-terminated, and the flags are bounded constants.
    let descriptor = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            mode,
        )
    };
    if descriptor < 0 {
        return Err(match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::ENOENT) => HostFailure::Unavailable,
            Some(libc::EEXIST) => HostFailure::Invalid,
            _ => HostFailure::Invalid,
        });
    }
    // SAFETY: a non-negative descriptor was returned exclusively to this call.
    Ok(unsafe { File::from_raw_fd(descriptor) })
}
