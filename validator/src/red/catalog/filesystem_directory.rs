use super::{RedCatalogError, error};
use std::ffi::{CStr, OsString};
use std::fs::File;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStringExt;

pub(super) fn directory_names(directory: &File) -> Result<Vec<OsString>, RedCatalogError> {
    // SAFETY: directory is live and fcntl duplicates it into one owned descriptor.
    let descriptor = unsafe { libc::fcntl(directory.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if descriptor < 0 {
        return Err(error(
            "red_catalog_fixture_directory_unreadable",
            "fixtures/red",
        ));
    }
    // SAFETY: fdopendir takes ownership of the duplicated directory descriptor.
    let stream = unsafe { libc::fdopendir(descriptor) };
    if stream.is_null() {
        // SAFETY: fdopendir failed, so this branch still owns descriptor.
        unsafe { libc::close(descriptor) };
        return Err(error(
            "red_catalog_fixture_directory_unreadable",
            "fixtures/red",
        ));
    }
    let stream = DirectoryStream(stream);
    let mut names = Vec::new();
    loop {
        clear_errno();
        // SAFETY: stream is live and owned by DirectoryStream.
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            if errno() != 0 {
                return Err(error(
                    "red_catalog_fixture_entry_unreadable",
                    "fixtures/red",
                ));
            }
            break;
        }
        // SAFETY: non-null dirent owns a NUL-terminated d_name until next readdir.
        let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if !matches!(bytes, b"." | b"..") {
            names.push(OsString::from_vec(bytes.to_vec()));
        }
    }
    names.sort();
    Ok(names)
}

struct DirectoryStream(*mut libc::DIR);

impl Drop for DirectoryStream {
    fn drop(&mut self) {
        // SAFETY: DirectoryStream exclusively owns this live stream.
        unsafe { libc::closedir(self.0) };
    }
}

#[cfg(target_os = "macos")]
fn clear_errno() {
    // SAFETY: __error returns current-thread errno storage.
    unsafe { *libc::__error() = 0 };
}

#[cfg(target_os = "macos")]
fn errno() -> i32 {
    // SAFETY: __error returns current-thread errno storage.
    unsafe { *libc::__error() }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn clear_errno() {
    // SAFETY: __errno_location returns current-thread errno storage.
    unsafe { *libc::__errno_location() = 0 };
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn errno() -> i32 {
    // SAFETY: __errno_location returns current-thread errno storage.
    unsafe { *libc::__errno_location() }
}
