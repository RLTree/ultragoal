use super::manifest;
pub use super::snapshot::{PackageEffects, PackageSnapshot, build_package, verify_package};
use super::source;
use super::spec::{self, ENTRY_LIMIT, PACKAGE_LIMIT, PackageRole};
use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::reader::{ReadSession, sha256};
use serde::Serialize;
use std::path::Path;

#[derive(Clone, Eq, PartialEq)]
pub struct PackageEntry {
    pub(crate) path: String,
    pub(crate) mode: u32,
    pub(crate) role: PackageRole,
    pub(crate) sha256: String,
    pub(crate) bytes: Vec<u8>,
}

impl std::fmt::Debug for PackageEntry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PackageEntry")
            .field("path", &self.path)
            .field("mode", &self.mode)
            .field("role", &self.role)
            .field("sha256", &self.sha256)
            .field("byte_length", &self.bytes.len())
            .finish()
    }
}

impl PackageEntry {
    pub fn path(&self) -> &str {
        &self.path
    }

    pub const fn mode(&self) -> u32 {
        self.mode
    }

    pub const fn role(&self) -> PackageRole {
        self.role
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn byte_length(&self) -> u64 {
        self.bytes.len() as u64
    }
}

#[derive(Clone)]
pub struct PackagePlan {
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) plugin_id: String,
    pub(crate) version: String,
    pub(crate) source_date_epoch: u64,
    pub(crate) catalog_id: String,
    pub(crate) accepted_inventory_sha256: String,
    pub(crate) source_tree_sha256: String,
    pub(crate) entries: Vec<PackageEntry>,
}

impl std::fmt::Debug for PackagePlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PackagePlan")
            .field("context_id", &self.context_id)
            .field("candidate_id", &self.candidate_id)
            .field("plugin_id", &self.plugin_id)
            .field("version", &self.version)
            .field("source_date_epoch", &self.source_date_epoch)
            .field("catalog_id", &self.catalog_id)
            .field("accepted_inventory_sha256", &self.accepted_inventory_sha256)
            .field("source_tree_sha256", &self.source_tree_sha256)
            .field("entries", &self.entries)
            .finish()
    }
}

impl PackagePlan {
    pub fn entries(&self) -> &[PackageEntry] {
        &self.entries
    }

    pub fn context_id(&self) -> &str {
        &self.context_id
    }

    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn accepted_inventory_sha256(&self) -> &str {
        &self.accepted_inventory_sha256
    }

    pub fn catalog_id(&self) -> &str {
        &self.catalog_id
    }

    pub fn source_tree_sha256(&self) -> &str {
        &self.source_tree_sha256
    }
}

pub fn plan_package(root: &Path, bytes: &[u8]) -> Result<PackagePlan, DistributionError> {
    let spec = spec::parse(bytes)?;
    let mut session = ReadSession::open(root)?;
    let mut total = 0usize;
    let mut entries = Vec::with_capacity(spec.entries.len());
    for row in spec.entries {
        let bytes = session.read(&row.source_path, ENTRY_LIMIT)?;
        total = total
            .checked_add(bytes.len())
            .ok_or_else(|| error(DistributionErrorId::ObjectTooLarge))?;
        if total > PACKAGE_LIMIT {
            return Err(error(DistributionErrorId::ObjectTooLarge));
        }
        entries.push(PackageEntry {
            path: row.path,
            mode: row.mode,
            role: row.role,
            sha256: sha256(&bytes),
            bytes,
        });
    }
    session.finish()?;
    manifest::validate(&entries, &spec.plugin_id, &spec.version)?;
    let source_tree_sha256 = entry_tree_sha256(&entries)?;
    Ok(PackagePlan {
        context_id: spec.context_id,
        candidate_id: spec.candidate_id,
        plugin_id: spec.plugin_id,
        version: spec.version,
        source_date_epoch: spec.source_date_epoch,
        catalog_id: source_tree_sha256.clone(),
        accepted_inventory_sha256: source_tree_sha256.clone(),
        source_tree_sha256,
        entries,
    })
}

pub fn plan_package_from_inventory(
    root: &Path,
    accepted_inventory_bytes: &[u8],
) -> Result<PackagePlan, DistributionError> {
    let source = source::load(root, accepted_inventory_bytes)?;
    manifest::validate(&source.entries, &source.plugin_id, &source.version)?;
    if entry_tree_sha256(&source.entries)? != source.source_tree_sha256 {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    Ok(PackagePlan {
        context_id: source.context_id,
        candidate_id: source.candidate_id,
        plugin_id: source.plugin_id,
        version: source.version,
        source_date_epoch: source.source_date_epoch,
        catalog_id: source.catalog_id,
        accepted_inventory_sha256: source.accepted_inventory_sha256,
        source_tree_sha256: source.source_tree_sha256,
        entries: source.entries,
    })
}

pub(crate) fn entry_tree_sha256(entries: &[PackageEntry]) -> Result<String, DistributionError> {
    #[derive(Serialize)]
    struct Row<'a> {
        path: &'a str,
        mode: u32,
        sha256: &'a str,
        byte_length: u64,
    }
    let rows = entries
        .iter()
        .map(|entry| Row {
            path: &entry.path,
            mode: entry.mode,
            sha256: &entry.sha256,
            byte_length: entry.bytes.len() as u64,
        })
        .collect::<Vec<_>>();
    serde_json::to_vec(&rows)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| error(DistributionErrorId::InvalidSpec))
}
