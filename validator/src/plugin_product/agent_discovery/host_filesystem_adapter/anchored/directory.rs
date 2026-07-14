use super::directory_syscall_adapter;
use super::identity::{
    anchored_directory, component, directory_fingerprint, directory_identity_sha256,
    file_fingerprint, require_regular_single_link, require_regular_single_link_at,
};
#[cfg(test)]
use super::probe_post_error_after_fault;
use super::scanner::DirectoryScanner;
use super::{AnchoredDirectory, SecureFile, record_file_open_attempt};
use crate::plugin_product::agent_discovery::error::AgentDiscoveryError;
use crate::plugin_product::agent_discovery::filesystem::{
    changed, digest, invalid_source, too_large, unsafe_entry,
};
use std::collections::BTreeSet;
use std::ffi::{CString, OsStr, OsString};
use std::fs::File;
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::{OsStrExt, OsStringExt};

const MAX_EXACT_DIRECTORY_ENTRIES: usize = 16;
const MAX_DIRECTORY_ENTRY_NAME_BYTES: usize = 96;

impl AnchoredDirectory {
    pub(crate) fn open_child(
        &self,
        name: &OsStr,
    ) -> Result<AnchoredDirectory, AgentDiscoveryError> {
        let name = component(name)?;
        let raw = directory_syscall_adapter::open_directory(self.handle.file.as_raw_fd(), &name)
            .map_err(|_| unsafe_entry())?;
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
                if observed.len() == expected.len() || !expected_bytes.contains(bytes.as_slice()) {
                    return Ok(false);
                }
                if !observed.insert(bytes) {
                    return Ok(false);
                }
            }
            Ok(observed == expected_bytes)
        })();
        #[cfg(test)]
        if scan.is_err() && probe_post_error_after_fault() && scanner.next().is_ok() {
            return Err(unsafe_entry());
        }
        scanner.close()?;
        self.revalidate()?;
        scan
    }

    pub(crate) fn bounded_regular_files(
        &self,
        maximum_entries: usize,
        maximum_file_bytes: usize,
    ) -> Result<Vec<(OsString, SecureFile)>, AgentDiscoveryError> {
        if maximum_entries == 0 || maximum_entries > 128 {
            return Err(invalid_source());
        }
        self.revalidate()?;
        let mut scanner = DirectoryScanner::open(self.handle.file.as_raw_fd())?;
        let scan = (|| {
            let mut names = BTreeSet::new();
            while let Some(bytes) = scanner.next()? {
                if bytes.is_empty() || bytes.contains(&b'/') || bytes.contains(&0) {
                    return Err(unsafe_entry());
                }
                if bytes.len() > MAX_DIRECTORY_ENTRY_NAME_BYTES || names.len() == maximum_entries {
                    return Err(too_large());
                }
                let name = OsString::from_vec(bytes);
                let component = component(&name)?;
                require_regular_single_link_at(self.handle.file.as_raw_fd(), &component)?;
                if !names.insert(name) {
                    return Err(unsafe_entry());
                }
            }
            let mut files = Vec::with_capacity(names.len());
            for name in names {
                files.push((name.clone(), self.read_file(&name, maximum_file_bytes)?));
            }
            Ok(files)
        })();
        scanner.close()?;
        self.revalidate()?;
        scan
    }

    pub(crate) fn identity_sha256(&self) -> String {
        directory_identity_sha256(b"anchored-child", &self.fingerprint)
    }

    pub(crate) fn read_file(
        &self,
        name: &OsStr,
        maximum: usize,
    ) -> Result<SecureFile, AgentDiscoveryError> {
        self.revalidate()?;
        let name = component(name)?;
        record_file_open_attempt();
        let raw = directory_syscall_adapter::open_regular_file(self.handle.file.as_raw_fd(), &name)
            .map_err(|_| unsafe_entry())?;
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
