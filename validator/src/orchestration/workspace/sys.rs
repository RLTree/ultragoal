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
    let duplicated = unsafe { libc::fcntl(directory.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if duplicated < 0 {
        return Err(OrchestrationError::IntegrationAmbiguous);
    }
    let stream = unsafe { libc::fdopendir(duplicated) };
    if stream.is_null() {
        unsafe { libc::close(duplicated) };
        return Err(OrchestrationError::IntegrationAmbiguous);
    }
    unsafe { libc::rewinddir(stream) };
    let mut alias = false;
    loop {
        let entry = unsafe { libc::readdir(stream) };
        if entry.is_null() {
            break;
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
        let Ok(name) = name.to_str() else {
            unsafe { libc::closedir(stream) };
            return Err(OrchestrationError::IntegrationAmbiguous);
        };
        if name == expected {
            unsafe { libc::closedir(stream) };
            return Ok(EntryMatch::Exact);
        }
        alias |= name.eq_ignore_ascii_case(expected);
    }
    unsafe { libc::closedir(stream) };
    Ok(if alias {
        EntryMatch::Alias
    } else {
        EntryMatch::Absent
    })
}

pub(super) fn duplicate(file: &File) -> Result<File, OrchestrationError> {
    let descriptor = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if descriptor < 0 {
        return Err(OrchestrationError::IntegrationAmbiguous);
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

pub(super) fn open_at(
    directory: &File,
    name: &str,
    flags: i32,
) -> Result<Option<File>, OrchestrationError> {
    let name = CString::new(name).map_err(|_| OrchestrationError::InvalidPath)?;
    let descriptor = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        return match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::ENOENT) => Ok(None),
            _ => Err(OrchestrationError::IntegrationAmbiguous),
        };
    }
    Ok(Some(unsafe { File::from_raw_fd(descriptor) }))
}

pub(super) fn stat_at(
    directory: &File,
    name: &str,
) -> Result<Option<PathStat>, OrchestrationError> {
    let name = CString::new(name).map_err(|_| OrchestrationError::InvalidPath)?;
    let mut stat = MaybeUninit::<libc::stat>::uninit();
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
    let stat = unsafe { stat.assume_init() };
    Ok(Some(PathStat {
        device: stat.st_dev as u64,
        inode: stat.st_ino as u64,
        links: stat.st_nlink as u64,
        regular: stat.st_mode & libc::S_IFMT == libc::S_IFREG,
    }))
}
