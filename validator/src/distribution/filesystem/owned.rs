#[cfg(unix)]
use super::descriptor::{
    Directory, FileIdentity, create_file, entry_matches_file, entry_matches_file_after_rename,
    unlink_file_identity,
};
use crate::distribution::error::{DistributionError, DistributionErrorId, error};

#[cfg(unix)]
pub(super) struct OwnedFile {
    parent: Directory,
    name: String,
    file: std::fs::File,
    identity: FileIdentity,
    mode: u32,
    active: bool,
    conflict_error: bool,
}

#[cfg(unix)]
impl OwnedFile {
    pub(super) fn create(
        parent: Directory,
        name: String,
        bytes: &[u8],
        mode: u32,
        conflict_error: bool,
    ) -> Result<Self, DistributionError> {
        let (file, identity) = create_file(&parent, &name, bytes, mode).map_err(|failure| {
            if conflict_error && failure.id() == DistributionErrorId::EffectFailed {
                error(DistributionErrorId::InstallConflict)
            } else {
                failure
            }
        })?;
        Ok(Self {
            parent,
            name,
            file,
            identity,
            mode,
            active: true,
            conflict_error,
        })
    }

    pub(super) fn require_current(&self) -> Result<(), DistributionError> {
        if !entry_matches_file(&self.parent, &self.name, &self.file, self.identity)? {
            return Err(error(if self.conflict_error {
                DistributionErrorId::InstallConflict
            } else {
                DistributionErrorId::ObjectChanged
            }));
        }
        Ok(())
    }

    pub(super) fn require_at(
        &self,
        parent: &Directory,
        name: &str,
    ) -> Result<(), DistributionError> {
        if !entry_matches_file_after_rename(parent, name, &self.file, self.identity)? {
            return Err(error(if self.conflict_error {
                DistributionErrorId::InstallConflict
            } else {
                DistributionErrorId::ObjectChanged
            }));
        }
        Ok(())
    }

    pub(super) fn identity(&self) -> FileIdentity {
        self.identity
    }

    pub(super) fn mode(&self) -> u32 {
        self.mode
    }

    pub(super) fn disarm(&mut self) {
        self.active = false;
    }
}

#[cfg(unix)]
impl Drop for OwnedFile {
    fn drop(&mut self) {
        let _ = self.file.sync_all();
        if self.active {
            let _ = unlink_file_identity(&self.parent, &self.name, self.identity);
        }
    }
}
