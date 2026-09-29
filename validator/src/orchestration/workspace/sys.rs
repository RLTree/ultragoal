use super::super::OrchestrationError;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum EntryMatch {
    Exact,
    Alias,
    Absent,
}

pub(super) struct PathStat {
    pub device: u64,
    pub inode: u64,
    pub links: u64,
    pub regular: bool,
}

pub(super) fn exact_entry(
    directory: &File,
    expected: &str,
) -> Result<EntryMatch, OrchestrationError> {
    // SAFETY: `directory` supplies a live descriptor borrowed for this call.
    let duplicated = unsafe { libc::fcntl(directory.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if duplicated < 0 {
        return Err(OrchestrationError::IntegrationAmbiguous);
    }
    // SAFETY: `duplicated` is owned and is transferred to `fdopendir` on success.
    let stream = unsafe { libc::fdopendir(duplicated) };
    if stream.is_null() {
        // SAFETY: `fdopendir` did not consume this owned descriptor.
        unsafe { libc::close(duplicated) };
        return Err(OrchestrationError::IntegrationAmbiguous);
    }
    // SAFETY: `stream` is a non-null directory stream until `closedir` below.
    unsafe { libc::rewinddir(stream) };
    let mut alias = false;
    loop {
        clear_readdir_error();
        // SAFETY: `stream` remains valid and is used by this thread only.
        let entry = unsafe { libc::readdir(stream) };
        if entry.is_null() {
            if readdir_failed() {
                // SAFETY: this branch closes the still-owned stream exactly once.
                unsafe { libc::closedir(stream) };
                return Err(OrchestrationError::IntegrationAmbiguous);
            }
            break;
        }
        // SAFETY: successful `readdir` returns a valid entry with NUL-terminated `d_name`.
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
        let Ok(name) = name.to_str() else {
            // SAFETY: this branch closes the still-owned stream exactly once.
            unsafe { libc::closedir(stream) };
            return Err(OrchestrationError::IntegrationAmbiguous);
        };
        if name == expected {
            // SAFETY: this branch closes the still-owned stream exactly once.
            unsafe { libc::closedir(stream) };
            return Ok(EntryMatch::Exact);
        }
        alias |= name.eq_ignore_ascii_case(expected);
    }
    // SAFETY: the loop has finished and this is the sole remaining stream owner.
    unsafe { libc::closedir(stream) };
    Ok(if alias {
        EntryMatch::Alias
    } else {
        EntryMatch::Absent
    })
}

#[cfg(target_os = "macos")]
fn clear_readdir_error() {
    // SAFETY: libc exposes thread-local errno storage for the current thread.
    unsafe { *libc::__error() = 0 };
}

#[cfg(target_os = "macos")]
fn readdir_failed() -> bool {
    // SAFETY: libc exposes thread-local errno storage for the current thread.
    readdir_failed_from_errno(unsafe { *libc::__error() })
}

#[cfg(target_os = "linux")]
fn clear_readdir_error() {
    // SAFETY: libc exposes thread-local errno storage for the current thread.
    unsafe { *libc::__errno_location() = 0 };
}

#[cfg(target_os = "linux")]
fn readdir_failed() -> bool {
    // SAFETY: libc exposes thread-local errno storage for the current thread.
    readdir_failed_from_errno(unsafe { *libc::__errno_location() })
}

#[cfg(all(unix, not(any(target_os = "macos", target_os = "linux"))))]
fn clear_readdir_error() {}

#[cfg(all(unix, not(any(target_os = "macos", target_os = "linux"))))]
fn readdir_failed() -> bool {
    true
}

fn readdir_failed_from_errno(errno: i32) -> bool {
    errno != 0
}

#[cfg(test)]
mod tests {
    use super::readdir_failed_from_errno;

    #[test]
    fn eof_is_not_a_directory_read_failure_but_errno_is() {
        assert!(!readdir_failed_from_errno(0));
        assert!(readdir_failed_from_errno(libc::EIO));
    }
}

pub(super) fn duplicate(file: &File) -> Result<File, OrchestrationError> {
    // SAFETY: `file` supplies a live descriptor borrowed for this call.
    let descriptor = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if descriptor < 0 {
        return Err(OrchestrationError::IntegrationAmbiguous);
    }
    // SAFETY: `fcntl` returned a new owned descriptor on this success path.
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

pub(super) fn open_at(
    directory: &File,
    name: &str,
    flags: i32,
) -> Result<Option<File>, OrchestrationError> {
    let name = CString::new(name).map_err(|_| OrchestrationError::InvalidPath)?;
    // SAFETY: `name` is NUL-terminated and `directory` supplies a live descriptor.
    let descriptor = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        return match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::ENOENT) => Ok(None),
            _ => Err(OrchestrationError::IntegrationAmbiguous),
        };
    }
    // SAFETY: `openat` returned a new owned descriptor on this success path.
    Ok(Some(unsafe { File::from_raw_fd(descriptor) }))
}

pub(super) fn stat_at(
    directory: &File,
    name: &str,
) -> Result<Option<PathStat>, OrchestrationError> {
    let name = CString::new(name).map_err(|_| OrchestrationError::InvalidPath)?;
    let mut stat = MaybeUninit::<libc::stat>::uninit();
    // SAFETY: `stat` is valid writable storage and `name` is NUL-terminated.
    let result = unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        return match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::ENOENT) => Ok(None),
            _ => Err(OrchestrationError::IntegrationAmbiguous),
        };
    }
    // SAFETY: successful `fstatat` initialized `stat` fully.
    let stat = unsafe { stat.assume_init() };
    Ok(Some(PathStat {
        device: stat.st_dev as u64,
        inode: stat.st_ino,
        links: stat.st_nlink as u64,
        regular: stat.st_mode & libc::S_IFMT == libc::S_IFREG,
    }))
}
