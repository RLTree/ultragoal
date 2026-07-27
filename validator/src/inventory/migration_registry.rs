use super::digest::sha256_hex;
use super::fs::read_bounded;
use super::{ActiveStatus, AuthorityState, InventoryEntry, InventoryError};
use crate::context::ReadSession;
use std::fs;
use std::path::Path;

pub(crate) const MIGRATION_REGISTRY_PATH: &str = "migration/authority-routes.json";
pub(crate) const MAX_MIGRATION_REGISTRY_BYTES: u64 = 2 * 1024 * 1024;

pub(crate) struct ObservedMigrationRegistry {
    bytes: Vec<u8>,
    source_identity_sha256: String,
    unix_mode: Option<u32>,
}

impl ObservedMigrationRegistry {
    pub(crate) fn observe(reads: &ReadSession, root: &Path) -> Result<Self, InventoryError> {
        let path = root.join(MIGRATION_REGISTRY_PATH);
        let metadata = fs::symlink_metadata(&path).map_err(|error| InventoryError::Io {
            path: path.clone(),
            message: error.to_string(),
        })?;
        if !metadata.is_file() || metadata.file_type().is_symlink() || link_count(&metadata) != 1 {
            return Err(InventoryError::InvalidRegistry(format!(
                "{MIGRATION_REGISTRY_PATH}: registry must be a singly linked regular file"
            )));
        }
        let bytes = read_bounded(reads, &path, MAX_MIGRATION_REGISTRY_BYTES)?;
        let source_identity_sha256 = format!("sha256:{}", sha256_hex(&bytes));
        Ok(Self {
            bytes,
            source_identity_sha256,
            unix_mode: unix_mode(&metadata),
        })
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) fn source_identity_sha256(&self) -> &str {
        &self.source_identity_sha256
    }

    pub(crate) fn inventory_entry(&self) -> InventoryEntry {
        InventoryEntry {
            stable_id: "AUTHORITY-ROUTING-REGISTRY".to_owned(),
            kind: "migration-authority-registry".to_owned(),
            owner_role: "OWN-ULTRA-ROOT".to_owned(),
            relative_path: MIGRATION_REGISTRY_PATH.to_owned(),
            digest_sha256: self.source_identity_sha256[7..].to_owned(),
            unix_mode: self.unix_mode,
            authority_state: AuthorityState::Canonical,
            active_status: ActiveStatus::Active,
            generator: None,
            input_provenance: vec!["docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/MIGRATION-AND-RETIREMENT.md".to_owned()],
            references: Vec::new(),
        }
    }
}

#[cfg(unix)]
fn link_count(metadata: &fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    metadata.nlink()
}

#[cfg(not(unix))]
fn link_count(_metadata: &fs::Metadata) -> u64 {
    1
}

#[cfg(unix)]
fn unix_mode(metadata: &fs::Metadata) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    Some(metadata.permissions().mode())
}

#[cfg(not(unix))]
fn unix_mode(_metadata: &fs::Metadata) -> Option<u32> {
    None
}
