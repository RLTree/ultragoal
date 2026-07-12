use super::error::{AgentDiscoveryError, AgentDiscoveryErrorId};
use super::model::{MAX_DESCRIPTOR_BYTES, ProjectAgentDescriptor};
use sha2::{Digest, Sha256};
use std::path::{Component, Path};

#[cfg(unix)]
mod anchored {
    use super::*;
    use std::collections::BTreeSet;
    use std::ffi::{CStr, CString, OsStr, OsString};
    use std::fs::{File, Metadata};
    use std::io::Read;
    use std::os::fd::{AsRawFd, FromRawFd, RawFd};
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::MetadataExt;
    use std::path::PathBuf;
    use std::sync::Arc;

    const MAX_EXACT_DIRECTORY_ENTRIES: usize = 16;
    const MAX_DIRECTORY_ENTRY_NAME_BYTES: usize = 96;

    #[cfg(test)]
    use std::cell::Cell;

    #[cfg(test)]
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub(crate) struct TestIoCounts {
        pub(crate) directory_entries: usize,
        pub(crate) directory_scans: usize,
        pub(crate) metadata_probes: usize,
        pub(crate) file_open_attempts: usize,
        pub(crate) readdir_calls: usize,
        pub(crate) readdir_errors: usize,
        pub(crate) failed_stream_closes: usize,
        pub(crate) post_terminal_calls: usize,
    }

    #[cfg(test)]
    thread_local! {
        static TEST_IO_COUNTS: Cell<TestIoCounts> = const { Cell::new(TestIoCounts {
            directory_entries: 0,
            directory_scans: 0,
            metadata_probes: 0,
            file_open_attempts: 0,
            readdir_calls: 0,
            readdir_errors: 0,
            failed_stream_closes: 0,
            post_terminal_calls: 0,
        }) };
    }

    #[cfg(test)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct TestReaddirFault {
        scan_ordinal: usize,
        after_entries: usize,
        error_number: libc::c_int,
        probe_post_error: bool,
        triggered: bool,
    }

    #[cfg(test)]
    thread_local! {
        static TEST_SCAN_ORDINAL: Cell<usize> = const { Cell::new(0) };
        static TEST_READDIR_FAULT: Cell<Option<TestReaddirFault>> = const { Cell::new(None) };
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) struct FileFingerprint {
        len: u64,
        modified_seconds: i64,
        modified_nanos: i64,
        changed_seconds: i64,
        changed_nanos: i64,
        device: u64,
        inode: u64,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) struct DirectoryFingerprint {
        modified_seconds: i64,
        modified_nanos: i64,
        changed_seconds: i64,
        changed_nanos: i64,
        device: u64,
        inode: u64,
    }

    #[derive(Debug)]
    struct DirectoryHandle {
        file: File,
    }

    struct DirectoryScanner {
        stream: *mut libc::DIR,
        scan_ordinal: usize,
        yielded_entries: usize,
        terminal: bool,
        failed: bool,
        closed: bool,
    }

    #[derive(Clone, Debug)]
    pub(crate) struct AnchoredDirectory {
        handle: Arc<DirectoryHandle>,
        fingerprint: DirectoryFingerprint,
    }

    #[derive(Clone, Debug)]
    pub(crate) struct AnchoredRoot {
        root_path: PathBuf,
        directory: AnchoredDirectory,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) struct SecureFile {
        pub(crate) bytes: Vec<u8>,
        pub(crate) sha256: String,
        pub(crate) fingerprint: FileFingerprint,
    }

    impl AnchoredRoot {
        pub(crate) fn open(path: &Path) -> Result<Self, AgentDiscoveryError> {
            let metadata = std::fs::symlink_metadata(path).map_err(|_| unsafe_entry())?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(unsafe_entry());
            }
            let canonical = std::fs::canonicalize(path).map_err(|_| unsafe_entry())?;
            let directory = open_directory_path(&canonical)?;
            let expected = directory_fingerprint(&metadata)?;
            if !same_directory_identity(&directory.fingerprint, &expected) {
                return Err(changed());
            }
            Ok(Self {
                root_path: canonical,
                directory,
            })
        }

        pub(crate) fn open_dir(
            &self,
            relative: &str,
        ) -> Result<AnchoredDirectory, AgentDiscoveryError> {
            if !safe_relative(relative) {
                return Err(unsafe_entry());
            }
            let mut current = self.directory.clone();
            for component in Path::new(relative).components() {
                let Component::Normal(name) = component else {
                    return Err(unsafe_entry());
                };
                current = current.open_child(name)?;
            }
            Ok(current)
        }

        pub(crate) fn canonical_path(&self) -> &Path {
            &self.root_path
        }

        pub(crate) fn revalidate(&self) -> Result<(), AgentDiscoveryError> {
            self.directory.revalidate()?;
            let current = open_directory_path(&self.root_path)?;
            if current.fingerprint != self.directory.fingerprint {
                return Err(changed());
            }
            Ok(())
        }

        pub(crate) fn revalidate_dir(
            &self,
            relative: &str,
            expected: &AnchoredDirectory,
        ) -> Result<(), AgentDiscoveryError> {
            let current = self.open_dir(relative)?;
            if current.fingerprint != expected.fingerprint {
                return Err(changed());
            }
            expected.revalidate()
        }
    }

    impl AnchoredDirectory {
        pub(crate) fn open_child(
            &self,
            name: &OsStr,
        ) -> Result<AnchoredDirectory, AgentDiscoveryError> {
            let name = component(name)?;
            let raw = unsafe {
                libc::openat(
                    self.handle.file.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if raw < 0 {
                return Err(unsafe_entry());
            }
            let file = unsafe { File::from_raw_fd(raw) };
            anchored_directory(file)
        }

        pub(crate) fn revalidate(&self) -> Result<(), AgentDiscoveryError> {
            let current =
                directory_fingerprint(&self.handle.file.metadata().map_err(|_| unsafe_entry())?)?;
            if current != self.fingerprint {
                return Err(changed());
            }
            Ok(())
        }

        pub(crate) fn exact_regular_entries(
            &self,
            expected: &BTreeSet<OsString>,
        ) -> Result<bool, AgentDiscoveryError> {
            if expected.is_empty() || expected.len() > MAX_EXACT_DIRECTORY_ENTRIES {
                return Err(invalid_source());
            }
            let expected_bytes = expected
                .iter()
                .map(|name| {
                    let bytes = name.as_bytes();
                    if bytes.len() > MAX_DIRECTORY_ENTRY_NAME_BYTES {
                        return Err(invalid_source());
                    }
                    component(name).map(|_| bytes.to_vec())
                })
                .collect::<Result<BTreeSet<_>, _>>()?;
            if expected_bytes.len() != expected.len() {
                return Err(invalid_source());
            }
            self.revalidate()?;
            let mut scanner = DirectoryScanner::open(self.handle.file.as_raw_fd())?;
            let scan = (|| {
                let mut observed = BTreeSet::new();
                while let Some(bytes) = scanner.next()? {
                    if bytes.is_empty() || bytes.contains(&b'/') || bytes.contains(&0) {
                        return Err(unsafe_entry());
                    }
                    if bytes.len() > MAX_DIRECTORY_ENTRY_NAME_BYTES {
                        return Err(too_large());
                    }
                    let name = CString::new(bytes.as_slice()).map_err(|_| unsafe_entry())?;
                    require_regular_single_link_at(self.handle.file.as_raw_fd(), &name)?;
                    if observed.len() == expected.len()
                        || !expected_bytes.contains(bytes.as_slice())
                    {
                        return Ok(false);
                    }
                    if !observed.insert(bytes) {
                        return Ok(false);
                    }
                }
                Ok(observed == expected_bytes)
            })();
            #[cfg(test)]
            if scan.is_err() && probe_post_error_after_fault() {
                if scanner.next().is_ok() {
                    return Err(unsafe_entry());
                }
            }
            scanner.close()?;
            self.revalidate()?;
            scan
        }

        pub(crate) fn read_file(
            &self,
            name: &OsStr,
            maximum: usize,
        ) -> Result<SecureFile, AgentDiscoveryError> {
            self.revalidate()?;
            let name = component(name)?;
            record_file_open_attempt();
            let raw = unsafe {
                libc::openat(
                    self.handle.file.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                )
            };
            if raw < 0 {
                return Err(unsafe_entry());
            }
            let mut file = unsafe { File::from_raw_fd(raw) };
            let before = file.metadata().map_err(|_| unsafe_entry())?;
            require_regular_single_link(&before, maximum)?;
            let before_fingerprint = file_fingerprint(&before)?;
            let mut bytes = Vec::with_capacity(before.len() as usize);
            file.by_ref()
                .take((maximum as u64).saturating_add(1))
                .read_to_end(&mut bytes)
                .map_err(|_| unsafe_entry())?;
            if bytes.len() > maximum {
                return Err(too_large());
            }
            let after = file.metadata().map_err(|_| unsafe_entry())?;
            require_regular_single_link(&after, maximum)?;
            let after_fingerprint = file_fingerprint(&after)?;
            if before_fingerprint != after_fingerprint || after.len() != bytes.len() as u64 {
                return Err(changed());
            }
            self.revalidate()?;
            Ok(SecureFile {
                sha256: digest(&bytes),
                bytes,
                fingerprint: after_fingerprint,
            })
        }

        pub(crate) fn same_file(
            &self,
            name: &OsStr,
            maximum: usize,
            expected: &SecureFile,
        ) -> Result<bool, AgentDiscoveryError> {
            Ok(self.read_file(name, maximum)? == *expected)
        }
    }

    impl DirectoryScanner {
        fn open(directory_fd: RawFd) -> Result<Self, AgentDiscoveryError> {
            let current = c".";
            let duplicate = unsafe {
                libc::openat(
                    directory_fd,
                    current.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if duplicate < 0 {
                return Err(unsafe_entry());
            }
            let stream = unsafe { libc::fdopendir(duplicate) };
            if stream.is_null() {
                unsafe { libc::close(duplicate) };
                return Err(unsafe_entry());
            }
            record_directory_scan();
            Ok(Self {
                stream,
                scan_ordinal: next_scan_ordinal(),
                yielded_entries: 0,
                terminal: false,
                failed: false,
                closed: false,
            })
        }

        fn next(&mut self) -> Result<Option<Vec<u8>>, AgentDiscoveryError> {
            if self.terminal || self.closed {
                record_post_terminal_call();
                return Err(unsafe_entry());
            }
            loop {
                set_errno(0)?;
                record_readdir_call();
                let entry = if let Some(error_number) =
                    injected_readdir_error(self.scan_ordinal, self.yielded_entries)
                {
                    set_errno(error_number)?;
                    std::ptr::null_mut()
                } else {
                    unsafe { libc::readdir(self.stream) }
                };
                if entry.is_null() {
                    let error_number = get_errno()?;
                    self.terminal = true;
                    if error_number == 0 {
                        return Ok(None);
                    }
                    self.failed = true;
                    record_readdir_error();
                    return Err(unsafe_entry());
                }
                let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
                if bytes == b"." || bytes == b".." {
                    continue;
                }
                self.yielded_entries += 1;
                record_directory_entry();
                return Ok(Some(bytes.to_vec()));
            }
        }

        fn close(&mut self) -> Result<(), AgentDiscoveryError> {
            if self.closed {
                return Err(unsafe_entry());
            }
            if self.failed {
                record_failed_stream_close();
            }
            self.closed = true;
            if unsafe { libc::closedir(self.stream) } != 0 {
                return Err(unsafe_entry());
            }
            Ok(())
        }
    }

    impl Drop for DirectoryScanner {
        fn drop(&mut self) {
            if !self.closed {
                if self.failed {
                    record_failed_stream_close();
                }
                self.closed = true;
                unsafe { libc::closedir(self.stream) };
            }
        }
    }

    fn set_errno(value: libc::c_int) -> Result<(), AgentDiscoveryError> {
        let pointer = errno_pointer();
        if pointer.is_null() {
            return Err(unsafe_entry());
        }
        unsafe { *pointer = value };
        Ok(())
    }

    fn get_errno() -> Result<libc::c_int, AgentDiscoveryError> {
        let pointer = errno_pointer();
        if pointer.is_null() {
            return Err(unsafe_entry());
        }
        Ok(unsafe { *pointer })
    }

    #[cfg(any(target_vendor = "apple", target_os = "freebsd"))]
    fn errno_pointer() -> *mut libc::c_int {
        unsafe { libc::__error() }
    }

    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "emscripten",
        target_os = "hurd",
        target_os = "redox"
    ))]
    fn errno_pointer() -> *mut libc::c_int {
        unsafe { libc::__errno_location() }
    }

    #[cfg(any(
        target_os = "android",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "cygwin",
        target_os = "nuttx"
    ))]
    fn errno_pointer() -> *mut libc::c_int {
        unsafe { libc::__errno() }
    }

    #[cfg(any(target_os = "illumos", target_os = "solaris"))]
    fn errno_pointer() -> *mut libc::c_int {
        unsafe { libc::___errno() }
    }

    #[cfg(not(any(
        target_vendor = "apple",
        target_os = "freebsd",
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "emscripten",
        target_os = "hurd",
        target_os = "redox",
        target_os = "android",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "cygwin",
        target_os = "nuttx",
        target_os = "illumos",
        target_os = "solaris"
    )))]
    fn errno_pointer() -> *mut libc::c_int {
        std::ptr::null_mut()
    }

    fn open_directory_path(path: &Path) -> Result<AnchoredDirectory, AgentDiscoveryError> {
        let path = CString::new(path.as_os_str().as_bytes()).map_err(|_| unsafe_entry())?;
        let raw = unsafe {
            libc::open(
                path.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if raw < 0 {
            return Err(unsafe_entry());
        }
        let file = unsafe { File::from_raw_fd(raw) };
        anchored_directory(file)
    }

    fn anchored_directory(file: File) -> Result<AnchoredDirectory, AgentDiscoveryError> {
        let fingerprint = directory_fingerprint(&file.metadata().map_err(|_| unsafe_entry())?)?;
        Ok(AnchoredDirectory {
            handle: Arc::new(DirectoryHandle { file }),
            fingerprint,
        })
    }

    fn component(name: &OsStr) -> Result<CString, AgentDiscoveryError> {
        let bytes = name.as_bytes();
        if bytes.is_empty() || bytes == b"." || bytes == b".." || bytes.contains(&b'/') {
            return Err(unsafe_entry());
        }
        CString::new(bytes).map_err(|_| unsafe_entry())
    }

    fn directory_fingerprint(
        metadata: &Metadata,
    ) -> Result<DirectoryFingerprint, AgentDiscoveryError> {
        if !metadata.is_dir() {
            return Err(unsafe_entry());
        }
        Ok(DirectoryFingerprint {
            modified_seconds: metadata.mtime(),
            modified_nanos: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanos: metadata.ctime_nsec(),
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }

    fn file_fingerprint(metadata: &Metadata) -> Result<FileFingerprint, AgentDiscoveryError> {
        if !metadata.is_file() {
            return Err(unsafe_entry());
        }
        Ok(FileFingerprint {
            len: metadata.len(),
            modified_seconds: metadata.mtime(),
            modified_nanos: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanos: metadata.ctime_nsec(),
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }

    fn same_directory_identity(left: &DirectoryFingerprint, right: &DirectoryFingerprint) -> bool {
        left.device == right.device && left.inode == right.inode
    }

    fn require_regular_single_link(
        metadata: &Metadata,
        maximum: usize,
    ) -> Result<(), AgentDiscoveryError> {
        if !metadata.is_file() || metadata.nlink() != 1 {
            return Err(unsafe_entry());
        }
        if metadata.len() > maximum as u64 {
            return Err(too_large());
        }
        Ok(())
    }

    fn require_regular_single_link_at(
        directory_fd: RawFd,
        name: &CStr,
    ) -> Result<(), AgentDiscoveryError> {
        record_metadata_probe();
        let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
        let result = unsafe {
            libc::fstatat(
                directory_fd,
                name.as_ptr(),
                metadata.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result != 0 {
            return Err(unsafe_entry());
        }
        let metadata = unsafe { metadata.assume_init() };
        if metadata.st_mode & libc::S_IFMT != libc::S_IFREG || metadata.st_nlink != 1 {
            return Err(unsafe_entry());
        }
        Ok(())
    }

    #[cfg(test)]
    fn record_directory_entry() {
        TEST_IO_COUNTS.with(|counts| {
            let mut current = counts.get();
            current.directory_entries += 1;
            counts.set(current);
        });
    }

    #[cfg(not(test))]
    fn record_directory_entry() {}

    #[cfg(test)]
    fn record_directory_scan() {
        TEST_IO_COUNTS.with(|counts| {
            let mut current = counts.get();
            current.directory_scans += 1;
            counts.set(current);
        });
    }

    #[cfg(not(test))]
    fn record_directory_scan() {}

    #[cfg(test)]
    fn record_metadata_probe() {
        TEST_IO_COUNTS.with(|counts| {
            let mut current = counts.get();
            current.metadata_probes += 1;
            counts.set(current);
        });
    }

    #[cfg(not(test))]
    fn record_metadata_probe() {}

    #[cfg(test)]
    fn record_file_open_attempt() {
        TEST_IO_COUNTS.with(|counts| {
            let mut current = counts.get();
            current.file_open_attempts += 1;
            counts.set(current);
        });
    }

    #[cfg(not(test))]
    fn record_file_open_attempt() {}

    #[cfg(test)]
    fn record_readdir_call() {
        TEST_IO_COUNTS.with(|counts| {
            let mut current = counts.get();
            current.readdir_calls += 1;
            counts.set(current);
        });
    }

    #[cfg(not(test))]
    fn record_readdir_call() {}

    #[cfg(test)]
    fn record_readdir_error() {
        TEST_IO_COUNTS.with(|counts| {
            let mut current = counts.get();
            current.readdir_errors += 1;
            counts.set(current);
        });
    }

    #[cfg(not(test))]
    fn record_readdir_error() {}

    #[cfg(test)]
    fn record_failed_stream_close() {
        TEST_IO_COUNTS.with(|counts| {
            let mut current = counts.get();
            current.failed_stream_closes += 1;
            counts.set(current);
        });
    }

    #[cfg(not(test))]
    fn record_failed_stream_close() {}

    #[cfg(test)]
    fn record_post_terminal_call() {
        TEST_IO_COUNTS.with(|counts| {
            let mut current = counts.get();
            current.post_terminal_calls += 1;
            counts.set(current);
        });
    }

    #[cfg(not(test))]
    fn record_post_terminal_call() {}

    #[cfg(test)]
    fn next_scan_ordinal() -> usize {
        TEST_SCAN_ORDINAL.with(|ordinal| {
            let current = ordinal.get();
            ordinal.set(current + 1);
            current
        })
    }

    #[cfg(not(test))]
    fn next_scan_ordinal() -> usize {
        0
    }

    #[cfg(test)]
    fn injected_readdir_error(scan_ordinal: usize, yielded_entries: usize) -> Option<libc::c_int> {
        TEST_READDIR_FAULT.with(|fault| {
            let mut current = fault.get()?;
            if current.triggered
                || current.scan_ordinal != scan_ordinal
                || current.after_entries != yielded_entries
            {
                return None;
            }
            current.triggered = true;
            fault.set(Some(current));
            Some(current.error_number)
        })
    }

    #[cfg(test)]
    fn probe_post_error_after_fault() -> bool {
        TEST_READDIR_FAULT.with(|fault| {
            fault
                .get()
                .is_some_and(|fault| fault.triggered && fault.probe_post_error)
        })
    }

    #[cfg(not(test))]
    fn injected_readdir_error(
        _scan_ordinal: usize,
        _yielded_entries: usize,
    ) -> Option<libc::c_int> {
        None
    }

    #[cfg(test)]
    pub(crate) fn reset_test_io_counts() {
        TEST_IO_COUNTS.with(|counts| counts.set(TestIoCounts::default()));
        TEST_SCAN_ORDINAL.with(|ordinal| ordinal.set(0));
        TEST_READDIR_FAULT.with(|fault| fault.set(None));
    }

    #[cfg(test)]
    pub(crate) fn test_io_counts() -> TestIoCounts {
        TEST_IO_COUNTS.with(Cell::get)
    }

    #[cfg(test)]
    pub(crate) fn set_test_readdir_fault(
        scan_ordinal: usize,
        after_entries: usize,
        error_number: libc::c_int,
        probe_post_error: bool,
    ) {
        assert!(matches!(error_number, libc::EIO | libc::EINTR));
        TEST_SCAN_ORDINAL.with(|ordinal| ordinal.set(0));
        TEST_READDIR_FAULT.with(|fault| {
            fault.set(Some(TestReaddirFault {
                scan_ordinal,
                after_entries,
                error_number,
                probe_post_error,
                triggered: false,
            }));
        });
    }

    #[cfg(test)]
    pub(crate) fn test_readdir_fault_triggered() -> bool {
        TEST_READDIR_FAULT.with(|fault| fault.get().is_some_and(|fault| fault.triggered))
    }

    #[allow(dead_code)]
    fn _raw_fd(directory: &AnchoredDirectory) -> RawFd {
        directory.handle.file.as_raw_fd()
    }
}

#[cfg(not(unix))]
mod anchored {
    use super::*;
    use std::ffi::{OsStr, OsString};

    #[derive(Clone, Debug)]
    pub(crate) struct AnchoredRoot;
    #[derive(Clone, Debug)]
    pub(crate) struct AnchoredDirectory;
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) struct SecureFile {
        pub(crate) bytes: Vec<u8>,
        pub(crate) sha256: String,
    }

    impl AnchoredRoot {
        pub(crate) fn open(_path: &Path) -> Result<Self, AgentDiscoveryError> {
            Err(unsafe_entry())
        }
        pub(crate) fn open_dir(
            &self,
            _relative: &str,
        ) -> Result<AnchoredDirectory, AgentDiscoveryError> {
            Err(unsafe_entry())
        }
        pub(crate) fn canonical_path(&self) -> &Path {
            Path::new("")
        }
        pub(crate) fn revalidate(&self) -> Result<(), AgentDiscoveryError> {
            Err(unsafe_entry())
        }
        pub(crate) fn revalidate_dir(
            &self,
            _relative: &str,
            _expected: &AnchoredDirectory,
        ) -> Result<(), AgentDiscoveryError> {
            Err(unsafe_entry())
        }
    }

    impl AnchoredDirectory {
        pub(crate) fn open_child(
            &self,
            _name: &OsStr,
        ) -> Result<AnchoredDirectory, AgentDiscoveryError> {
            Err(unsafe_entry())
        }
        pub(crate) fn revalidate(&self) -> Result<(), AgentDiscoveryError> {
            Err(unsafe_entry())
        }
        pub(crate) fn exact_regular_entries(
            &self,
            _expected: &std::collections::BTreeSet<OsString>,
        ) -> Result<bool, AgentDiscoveryError> {
            Err(unsafe_entry())
        }
        pub(crate) fn read_file(
            &self,
            _name: &OsStr,
            _maximum: usize,
        ) -> Result<SecureFile, AgentDiscoveryError> {
            Err(unsafe_entry())
        }
        pub(crate) fn same_file(
            &self,
            _name: &OsStr,
            _maximum: usize,
            _expected: &SecureFile,
        ) -> Result<bool, AgentDiscoveryError> {
            Err(unsafe_entry())
        }
    }
}

pub(crate) use anchored::{AnchoredDirectory, AnchoredRoot, SecureFile};
#[cfg(all(test, unix))]
pub(crate) use anchored::{
    TestIoCounts, reset_test_io_counts, set_test_readdir_fault, test_io_counts,
    test_readdir_fault_triggered,
};

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(crate) fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    })
}

pub(crate) fn safe_relative(path: &str) -> bool {
    let path = Path::new(path);
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(value) if !value.is_empty()))
}

pub(crate) fn parse_descriptor(
    bytes: &[u8],
) -> Result<ProjectAgentDescriptor, AgentDiscoveryError> {
    if bytes.len() > MAX_DESCRIPTOR_BYTES {
        return Err(too_large());
    }
    let text = std::str::from_utf8(bytes).map_err(|_| invalid_source())?;
    let descriptor: ProjectAgentDescriptor = toml::from_str(text).map_err(|_| invalid_source())?;
    if !safe_name(&descriptor.name)
        || !safe_text(&descriptor.description, 1024, false)
        || !safe_text(&descriptor.developer_instructions, 16 * 1024, true)
    {
        return Err(invalid_source());
    }
    Ok(descriptor)
}

fn safe_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}

fn safe_text(value: &str, maximum: usize, multiline: bool) -> bool {
    !value.trim().is_empty()
        && value.len() <= maximum
        && !value.chars().any(|character| {
            character.is_control() && !(multiline && matches!(character, '\n' | '\r' | '\t'))
        })
}

fn unsafe_entry() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::UnsafeFilesystemEntry)
}

fn invalid_source() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::InvalidSourceCatalog)
}

fn changed() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::ObservationChanged)
}

fn too_large() -> AgentDiscoveryError {
    AgentDiscoveryError::new(AgentDiscoveryErrorId::InputTooLarge)
}
