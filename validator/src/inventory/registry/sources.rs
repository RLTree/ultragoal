use super::CONTRACT_DIR;
use crate::context::ReadSession;
use crate::inventory::fs::contract_source_entry;
use crate::inventory::types::{InventoryEntry, InventoryError};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const MAX_CONTRACT_SOURCE_FILES: usize = 256;

pub(super) fn load(
    reads: &ReadSession,
    root: &Path,
    entries: &mut Vec<InventoryEntry>,
) -> Result<usize, InventoryError> {
    let explicitly_loaded = [
        "PRODUCT_SURFACE_INVENTORY.json",
        "CUSTOM_TOOL_INVENTORY.json",
        "CLAIM_REGISTRY.json",
        "REQUIREMENT_TRACE.json",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let directory = root.join(CONTRACT_DIR);
    reads
        .pin_directory(&directory)
        .map_err(|error| InventoryError::Io {
            path: directory.clone(),
            message: error.to_string(),
        })?;
    let mut contract_paths = Vec::new();
    for entry in fs::read_dir(&directory).map_err(|error| InventoryError::Io {
        path: directory.clone(),
        message: error.to_string(),
    })? {
        reads.charge_entry().map_err(|error| InventoryError::Io {
            path: directory.clone(),
            message: error.to_string(),
        })?;
        let path = entry
            .map_err(|error| InventoryError::Io {
                path: directory.clone(),
                message: error.to_string(),
            })?
            .path();
        contract_paths.push(path);
        if contract_paths.len() > MAX_CONTRACT_SOURCE_FILES {
            return Err(InventoryError::InvalidRegistry(format!(
                "canonical contract contains more than {MAX_CONTRACT_SOURCE_FILES} entries"
            )));
        }
    }
    contract_paths.sort();
    let mut count = explicitly_loaded.len();
    for path in contract_paths {
        let metadata = fs::symlink_metadata(&path).map_err(|error| InventoryError::Io {
            path: path.clone(),
            message: error.to_string(),
        })?;
        if metadata.file_type().is_symlink() {
            return Err(InventoryError::InvalidRegistry(
                "canonical contract source cannot be a symlink".to_owned(),
            ));
        }
        if !metadata.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                InventoryError::InvalidRegistry(
                    "canonical contract source has a non-UTF-8 name".to_owned(),
                )
            })?;
        if explicitly_loaded.contains(name) {
            continue;
        }
        entries.push(contract_source_entry(reads, root, &path, name)?);
        count += 1;
    }
    let handoff_manifest = directory
        .parent()
        .expect("contract directory has bundle parent")
        .join("FINAL-HANDOFF-MANIFEST.sha256");
    let handoff_metadata =
        fs::symlink_metadata(&handoff_manifest).map_err(|error| InventoryError::Io {
            path: handoff_manifest.clone(),
            message: error.to_string(),
        })?;
    if !handoff_metadata.is_file() || handoff_metadata.file_type().is_symlink() {
        return Err(InventoryError::InvalidRegistry(
            "FINAL-HANDOFF-MANIFEST.sha256 must be a regular non-symlink file".to_owned(),
        ));
    }
    entries.push(contract_source_entry(
        reads,
        root,
        &handoff_manifest,
        "FINAL-HANDOFF-MANIFEST.sha256",
    )?);
    Ok(count + 1)
}
