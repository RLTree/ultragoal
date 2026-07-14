use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EntryMatch {
    Exact,
    Alias,
    Absent,
}

// Exact-name checks must inspect the entire directory: stopping at the first
// exact name would miss a case-fold alias later in the stream.  Bound both a
// single scan and all scans performed for one read so an untrusted repository
// cannot turn that correctness requirement into unbounded work.
pub(crate) const MAX_ENUMERATED_ENTRIES_PER_SCAN: usize = 256;
pub(crate) const MAX_ENUMERATED_NAME_BYTES_PER_SCAN: usize = 64 * 1024;
pub(crate) const MAX_ENUMERATED_ENTRIES_PER_OPERATION: usize = 4 * 1024;
pub(crate) const MAX_ENUMERATED_NAME_BYTES_PER_OPERATION: usize = 1024 * 1024;

pub(crate) struct EnumerationBudget {
    pub(crate) entries_remaining: usize,
    pub(crate) name_bytes_remaining: usize,
}

impl EnumerationBudget {
    pub(crate) const fn new() -> Self {
        Self {
            entries_remaining: MAX_ENUMERATED_ENTRIES_PER_OPERATION,
            name_bytes_remaining: MAX_ENUMERATED_NAME_BYTES_PER_OPERATION,
        }
    }

    pub(crate) fn observe(
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
pub(crate) struct EntryNames {
    pub(crate) exact_count: u8,
    pub(crate) alias: bool,
}

impl EntryNames {
    pub(crate) fn observe(&mut self, expected: &str, name: &str) {
        if name == expected {
            self.exact_count = self.exact_count.saturating_add(1);
        } else if name.eq_ignore_ascii_case(expected) {
            self.alias = true;
        }
    }

    pub(crate) fn classify(self) -> EntryMatch {
        if self.alias || self.exact_count > 1 {
            EntryMatch::Alias
        } else if self.exact_count == 1 {
            EntryMatch::Exact
        } else {
            EntryMatch::Absent
        }
    }
}

pub(crate) trait EntrySource {
    fn next(&mut self) -> Result<Option<&[u8]>, FitError>;
    fn close(&mut self) -> Result<(), FitError>;
}

pub(crate) struct DirectoryEntries {
    pub(crate) stream: Option<*mut libc::DIR>,
}

impl DirectoryEntries {
    pub(crate) fn open(directory: &File) -> Result<Self, FitError> {
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
pub(crate) struct PathStat {
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

pub(crate) fn exact_entry(
    directory: &File,
    expected: &str,
    budget: &mut EnumerationBudget,
) -> Result<EntryMatch, FitError> {
    let mut entries = DirectoryEntries::open(directory)?;
    enumerate_and_close(&mut entries, expected, budget)
}

pub(crate) fn enumerate_and_close<S: EntrySource>(
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
