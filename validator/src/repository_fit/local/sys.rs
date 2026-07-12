use super::super::{FitError, FitErrorId, error};
use std::ffi::{CStr, CString};
use std::fs::File;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum EntryMatch {
    Exact,
    Alias,
    Absent,
}

// Exact-name checks must inspect the entire directory: stopping at the first
// exact name would miss a case-fold alias later in the stream.  Bound both a
// single scan and all scans performed for one read so an untrusted repository
// cannot turn that correctness requirement into unbounded work.
pub(super) const MAX_ENUMERATED_ENTRIES_PER_SCAN: usize = 256;
pub(super) const MAX_ENUMERATED_NAME_BYTES_PER_SCAN: usize = 64 * 1024;
pub(super) const MAX_ENUMERATED_ENTRIES_PER_OPERATION: usize = 4 * 1024;
pub(super) const MAX_ENUMERATED_NAME_BYTES_PER_OPERATION: usize = 1024 * 1024;

pub(super) struct EnumerationBudget {
    entries_remaining: usize,
    name_bytes_remaining: usize,
}

impl EnumerationBudget {
    pub(super) const fn new() -> Self {
        Self {
            entries_remaining: MAX_ENUMERATED_ENTRIES_PER_OPERATION,
            name_bytes_remaining: MAX_ENUMERATED_NAME_BYTES_PER_OPERATION,
        }
    }

    fn observe(
        &mut self,
        scan_entries: usize,
        scan_name_bytes: usize,
        entry_name_bytes: usize,
    ) -> Result<(), FitError> {
        if scan_entries > MAX_ENUMERATED_ENTRIES_PER_SCAN
            || scan_name_bytes > MAX_ENUMERATED_NAME_BYTES_PER_SCAN
            || self.entries_remaining == 0
            || entry_name_bytes > self.name_bytes_remaining
        {
            return Err(error(FitErrorId::ResourceLimit));
        }
        self.entries_remaining -= 1;
        self.name_bytes_remaining -= entry_name_bytes;
        Ok(())
    }
}

#[derive(Default)]
struct EntryNames {
    exact_count: u8,
    alias: bool,
}

impl EntryNames {
    fn observe(&mut self, expected: &str, name: &str) {
        if name == expected {
            self.exact_count = self.exact_count.saturating_add(1);
        } else if name.eq_ignore_ascii_case(expected) {
            self.alias = true;
        }
    }

    fn classify(self) -> EntryMatch {
        if self.alias || self.exact_count > 1 {
            EntryMatch::Alias
        } else if self.exact_count == 1 {
            EntryMatch::Exact
        } else {
            EntryMatch::Absent
        }
    }
}

trait EntrySource {
    fn next(&mut self) -> Result<Option<&[u8]>, FitError>;
    fn close(&mut self) -> Result<(), FitError>;
}

struct DirectoryEntries {
    stream: Option<*mut libc::DIR>,
}

impl DirectoryEntries {
    fn open(directory: &File) -> Result<Self, FitError> {
        let duplicated = unsafe { libc::fcntl(directory.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
        if duplicated < 0 {
            return Err(error(FitErrorId::ReadFailed));
        }
        let stream = unsafe { libc::fdopendir(duplicated) };
        if stream.is_null() {
            unsafe { libc::close(duplicated) };
            return Err(error(FitErrorId::ReadFailed));
        }
        unsafe { libc::rewinddir(stream) };
        Ok(Self {
            stream: Some(stream),
        })
    }
}

impl EntrySource for DirectoryEntries {
    fn next(&mut self) -> Result<Option<&[u8]>, FitError> {
        let stream = self.stream.ok_or_else(|| error(FitErrorId::ReadFailed))?;
        clear_readdir_error()?;
        let entry = unsafe { libc::readdir(stream) };
        if entry.is_null() {
            return if readdir_error()? == 0 {
                Ok(None)
            } else {
                Err(error(FitErrorId::ReadFailed))
            };
        }
        Ok(Some(
            unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes(),
        ))
    }

    fn close(&mut self) -> Result<(), FitError> {
        let Some(stream) = self.stream.take() else {
            return Ok(());
        };
        if unsafe { libc::closedir(stream) } != 0 {
            return Err(error(FitErrorId::ReadFailed));
        }
        Ok(())
    }
}

impl Drop for DirectoryEntries {
    fn drop(&mut self) {
        if let Some(stream) = self.stream.take() {
            unsafe { libc::closedir(stream) };
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PathStat {
    pub device: u64,
    pub inode: u64,
    pub links: u64,
    pub length: u64,
    pub modified_seconds: i64,
    pub modified_nanoseconds: i64,
    pub changed_seconds: i64,
    pub changed_nanoseconds: i64,
    pub regular: bool,
}

pub(super) fn exact_entry(
    directory: &File,
    expected: &str,
    budget: &mut EnumerationBudget,
) -> Result<EntryMatch, FitError> {
    let mut entries = DirectoryEntries::open(directory)?;
    enumerate_and_close(&mut entries, expected, budget)
}

fn enumerate_and_close<S: EntrySource>(
    entries: &mut S,
    expected: &str,
    budget: &mut EnumerationBudget,
) -> Result<EntryMatch, FitError> {
    let observed = enumerate(entries, expected, budget);
    let closed = entries.close();
    match (observed, closed) {
        (Err(failure), _) => Err(failure),
        (Ok(_), Err(failure)) => Err(failure),
        (Ok(classification), Ok(())) => Ok(classification),
    }
}

fn enumerate<S: EntrySource>(
    entries: &mut S,
    expected: &str,
    budget: &mut EnumerationBudget,
) -> Result<EntryMatch, FitError> {
    let mut names = EntryNames::default();
    let mut entry_count: usize = 0;
    let mut name_bytes: usize = 0;
    loop {
        let observation = entries.next();
        let entry_name_bytes = match &observation {
            Ok(None) => break,
            Ok(Some(name)) => name.len(),
            Err(_) => 0,
        };
        entry_count = entry_count
            .checked_add(1)
            .ok_or_else(|| error(FitErrorId::ResourceLimit))?;
        name_bytes = name_bytes
            .checked_add(entry_name_bytes)
            .ok_or_else(|| error(FitErrorId::ResourceLimit))?;
        budget.observe(entry_count, name_bytes, entry_name_bytes)?;
        let name = observation?;
        let Some(name) = name else {
            unreachable!("normal EOF is handled before budget accounting")
        };
        let name = std::str::from_utf8(name).map_err(|_| error(FitErrorId::UnsafeObject))?;
        names.observe(expected, name);
    }
    Ok(names.classify())
}

#[cfg(target_vendor = "apple")]
fn errno_location() -> *mut libc::c_int {
    unsafe { libc::__error() }
}

#[cfg(target_os = "linux")]
fn errno_location() -> *mut libc::c_int {
    unsafe { libc::__errno_location() }
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
fn clear_readdir_error() -> Result<(), FitError> {
    unsafe { *errno_location() = 0 };
    Ok(())
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
fn readdir_error() -> Result<i32, FitError> {
    Ok(unsafe { *errno_location() })
}

#[cfg(not(any(target_vendor = "apple", target_os = "linux")))]
fn clear_readdir_error() -> Result<(), FitError> {
    Err(error(FitErrorId::UnsupportedHost))
}

#[cfg(not(any(target_vendor = "apple", target_os = "linux")))]
fn readdir_error() -> Result<i32, FitError> {
    Err(error(FitErrorId::UnsupportedHost))
}

pub(super) fn duplicate(file: &File) -> Result<File, FitError> {
    let descriptor = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if descriptor < 0 {
        return Err(error(FitErrorId::ReadFailed));
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

pub(super) fn open_at(directory: &File, name: &str, flags: i32) -> Result<Option<File>, FitError> {
    let name = CString::new(name).map_err(|_| error(FitErrorId::InvalidPath))?;
    let descriptor = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        return match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::ENOENT) => Ok(None),
            _ => Err(error(FitErrorId::UnsafeObject)),
        };
    }
    Ok(Some(unsafe { File::from_raw_fd(descriptor) }))
}

pub(super) fn stat_at(directory: &File, name: &str) -> Result<Option<PathStat>, FitError> {
    let name = CString::new(name).map_err(|_| error(FitErrorId::InvalidPath))?;
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
            _ => Err(error(FitErrorId::ReadFailed)),
        };
    }
    let stat = unsafe { stat.assume_init() };
    Ok(Some(PathStat {
        device: stat.st_dev as u64,
        inode: stat.st_ino as u64,
        links: stat.st_nlink as u64,
        length: stat.st_size as u64,
        modified_seconds: stat.st_mtime,
        modified_nanoseconds: stat.st_mtime_nsec,
        changed_seconds: stat.st_ctime,
        changed_nanoseconds: stat.st_ctime_nsec,
        regular: stat.st_mode & libc::S_IFMT == libc::S_IFREG,
    }))
}

#[cfg(test)]
#[path = "sys_tests.rs"]
mod tests;
