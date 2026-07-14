use super::identity::{
    directory_fingerprint, directory_identity_sha256, open_directory_path, same_directory_identity,
};
use super::{AnchoredDirectory, AnchoredRoot};
use crate::plugin_product::agent_discovery::error::AgentDiscoveryError;
use crate::plugin_product::agent_discovery::filesystem::{changed, safe_relative, unsafe_entry};
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path};

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

    pub(crate) fn identity_sha256(&self) -> String {
        directory_identity_sha256(
            self.root_path.as_os_str().as_bytes(),
            &self.directory.fingerprint,
        )
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
