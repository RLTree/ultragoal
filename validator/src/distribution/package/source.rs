use super::plan::PackageEntry;
use super::spec::{ENTRY_LIMIT, PACKAGE_LIMIT, PackageRole};
use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::json;
use crate::distribution::reader::{ReadSession, sha256, validate_relative_path};
use crate::distribution::spec::digest;
use crate::plugin_manifest::Version;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

const INVENTORY_LIMIT: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RawInventory {
    schema: String,
    context_id: String,
    candidate_id: String,
    catalog_id: String,
    plugin_id: String,
    version: String,
    source_date_epoch: u64,
    entries: Vec<RawEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RawEntry {
    path: String,
    object_type: ObjectType,
    mode: u32,
    sha256: String,
    byte_length: u64,
    role: PackageRole,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ObjectType {
    RegularFile,
}

pub(crate) struct SourceSet {
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

pub(crate) fn load(root: &Path, bytes: &[u8]) -> Result<SourceSet, DistributionError> {
    let raw: RawInventory = json::parse(bytes, INVENTORY_LIMIT)?;
    if raw.schema != "harness-ultragoal.accepted-package-source-set.v1"
        || !digest(&raw.context_id)
        || !digest(&raw.candidate_id)
        || !digest(&raw.catalog_id)
        || raw.plugin_id != "harness-ultragoal"
        || Version::parse(&raw.version).is_none()
        || raw.entries.is_empty()
        || raw.entries.len() > 4096
    {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    let mut paths = BTreeSet::new();
    let mut prior = None;
    for row in &raw.entries {
        validate_row(row, &mut paths, prior.as_deref())?;
        prior = Some(row.path.clone());
    }
    if serde_json::to_vec(&raw).map_err(|_| error(DistributionErrorId::InvalidSpec))? != bytes {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    let mut total = 0usize;
    let mut session = ReadSession::open(root)?;
    let mut entries = Vec::with_capacity(raw.entries.len());
    for row in &raw.entries {
        let metadata_before = source_metadata(root, &row.path)?;
        let object = session.read(&row.path, ENTRY_LIMIT)?;
        let metadata_after = source_metadata(root, &row.path)?;
        if metadata_before != metadata_after
            || metadata_after.mode != row.mode
            || object.len() as u64 != row.byte_length
            || sha256(&object) != row.sha256
        {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        total = total
            .checked_add(object.len())
            .ok_or_else(|| error(DistributionErrorId::ObjectTooLarge))?;
        if total > PACKAGE_LIMIT {
            return Err(error(DistributionErrorId::ObjectTooLarge));
        }
        entries.push(PackageEntry {
            path: row.path.clone(),
            mode: row.mode,
            role: row.role,
            sha256: row.sha256.clone(),
            bytes: object,
        });
    }
    session.finish()?;
    #[derive(Serialize)]
    struct TreeRow<'a> {
        path: &'a str,
        mode: u32,
        sha256: &'a str,
        byte_length: u64,
    }
    let tree_rows = raw
        .entries
        .iter()
        .map(|row| TreeRow {
            path: &row.path,
            mode: row.mode,
            sha256: &row.sha256,
            byte_length: row.byte_length,
        })
        .collect::<Vec<_>>();
    let tree_bytes =
        serde_json::to_vec(&tree_rows).map_err(|_| error(DistributionErrorId::InvalidSpec))?;
    Ok(SourceSet {
        context_id: raw.context_id,
        candidate_id: raw.candidate_id,
        plugin_id: raw.plugin_id,
        version: raw.version,
        source_date_epoch: raw.source_date_epoch,
        catalog_id: raw.catalog_id,
        accepted_inventory_sha256: sha256(bytes),
        source_tree_sha256: sha256(&tree_bytes),
        entries,
    })
}

fn validate_row(
    row: &RawEntry,
    paths: &mut BTreeSet<String>,
    prior: Option<&str>,
) -> Result<(), DistributionError> {
    validate_relative_path(&row.path)?;
    let folded = row.path.to_ascii_lowercase();
    if prior.is_some_and(|value| value >= row.path.as_str())
        || paths.iter().any(|path| {
            path == &folded
                || folded.starts_with(&(path.to_owned() + "/"))
                || path.starts_with(&(folded.clone() + "/"))
        })
        || !paths.insert(folded)
        || !digest(&row.sha256)
        || row.byte_length > ENTRY_LIMIT as u64
        || !matches!(row.mode, 0o644 | 0o755)
        || (row.role == PackageRole::Executable) != (row.mode == 0o755)
        || (row.path == ".codex-plugin/plugin.json") != (row.role == PackageRole::Manifest)
    {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    Ok(())
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct SourceMetadata {
    device: u64,
    inode: u64,
    mode: u32,
}

#[cfg(unix)]
fn source_metadata(root: &Path, path: &str) -> Result<SourceMetadata, DistributionError> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::symlink_metadata(root.join(path))
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
    if !metadata.is_file() || metadata.nlink() != 1 {
        return Err(error(DistributionErrorId::UnsafeObject));
    }
    Ok(SourceMetadata {
        device: metadata.dev(),
        inode: metadata.ino(),
        mode: metadata.mode() & 0o777,
    })
}

#[cfg(not(unix))]
fn source_metadata(root: &Path, path: &str) -> Result<SourceMetadata, DistributionError> {
    let metadata = std::fs::symlink_metadata(root.join(path))
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
    if !metadata.is_file() {
        return Err(error(DistributionErrorId::UnsafeObject));
    }
    Ok(SourceMetadata {
        device: 0,
        inode: 0,
        mode: 0o644,
    })
}
