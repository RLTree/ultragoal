use super::{digest, unsafe_entry};
use crate::plugin_product::agent_discovery::error::AgentDiscoveryError;
use std::ffi::{OsStr, OsString};
use std::path::Path;

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
    pub(crate) fn identity_sha256(&self) -> String {
        digest(b"unsupported-root")
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
    pub(crate) fn bounded_regular_files(
        &self,
        _maximum_entries: usize,
        _maximum_file_bytes: usize,
    ) -> Result<Vec<(OsString, SecureFile)>, AgentDiscoveryError> {
        Err(unsafe_entry())
    }
    pub(crate) fn identity_sha256(&self) -> String {
        digest(b"unsupported-directory")
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
