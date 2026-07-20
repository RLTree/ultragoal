use super::directory::Directory;
use super::types::{component, last_errno};
use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use std::ffi::CStr;
use std::os::fd::AsRawFd;

const DIRECTORY_ENTRY_LIMIT: usize = 4096;
const DIRECTORY_NAME_BYTES_LIMIT: usize = 1024 * 1024;

pub(super) fn names(directory: &Directory) -> Result<Vec<String>, DistributionError> {
    let parent = directory.current_descriptor()?;
    // SAFETY: `parent` is a live directory descriptor and `.` is NUL-terminated.
    let duplicate = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            c".".as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if duplicate < 0 {
        return Err(error(DistributionErrorId::ObjectUnavailable));
    }
    // SAFETY: `duplicate` is an owned directory descriptor returned by openat.
    let stream = unsafe { libc::fdopendir(duplicate) };
    if stream.is_null() {
        // SAFETY: fdopendir did not consume `duplicate` when it failed.
        unsafe { libc::close(duplicate) };
        return Err(error(DistributionErrorId::ObjectUnavailable));
    }
    struct Stream(*mut libc::DIR);
    impl Drop for Stream {
        fn drop(&mut self) {
            // SAFETY: Stream owns the non-null DIR pointer returned by fdopendir.
            unsafe { libc::closedir(self.0) };
        }
    }
    let stream = Stream(stream);
    let mut names = Vec::new();
    let mut name_bytes = 0usize;
    loop {
        clear_errno();
        // SAFETY: `stream.0` remains owned and valid until Stream is dropped.
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            if last_errno().is_some_and(|value| value != 0) {
                return Err(error(DistributionErrorId::ObjectUnavailable));
            }
            break;
        }
        // SAFETY: readdir returned a non-null entry whose d_name is NUL-terminated.
        let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if matches!(bytes, b"." | b"..") {
            continue;
        }
        name_bytes = name_bytes
            .checked_add(bytes.len())
            .ok_or_else(|| error(DistributionErrorId::ObjectTooLarge))?;
        if names.len() >= DIRECTORY_ENTRY_LIMIT || name_bytes > DIRECTORY_NAME_BYTES_LIMIT {
            return Err(error(DistributionErrorId::ObjectTooLarge));
        }
        let name = String::from_utf8(bytes.to_vec())
            .map_err(|_| error(DistributionErrorId::InvalidPath))?;
        component(&name)?;
        names.push(name);
    }
    names.sort();
    Ok(names)
}

#[cfg(target_os = "linux")]
fn clear_errno() {
    // SAFETY: __errno_location returns writable thread-local errno storage.
    unsafe { *libc::__errno_location() = 0 };
}

#[cfg(not(target_os = "linux"))]
fn clear_errno() {
    // SAFETY: __error returns writable thread-local errno storage.
    unsafe { *libc::__error() = 0 };
}
