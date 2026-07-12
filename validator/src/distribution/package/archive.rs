use super::plan::{PackageEntry, PackagePlan};
use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::reader::sha256;
use serde::Serialize;

const MAGIC: &[u8; 8] = b"HUGPKG1\0";

pub(crate) fn encode(plan: &PackagePlan) -> Result<Vec<u8>, DistributionError> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    push_string(&mut bytes, &plan.context_id)?;
    push_string(&mut bytes, &plan.candidate_id)?;
    push_string(&mut bytes, &plan.plugin_id)?;
    push_string(&mut bytes, &plan.version)?;
    push_string(&mut bytes, &plan.catalog_id)?;
    push_string(&mut bytes, &plan.accepted_inventory_sha256)?;
    push_string(&mut bytes, &plan.source_tree_sha256)?;
    bytes.extend_from_slice(&plan.source_date_epoch.to_be_bytes());
    bytes.extend_from_slice(&(plan.entries.len() as u32).to_be_bytes());
    for entry in &plan.entries {
        push_string(&mut bytes, &entry.path)?;
        bytes.extend_from_slice(&entry.mode.to_be_bytes());
        bytes.push(entry.role.code());
        bytes.extend_from_slice(&(entry.bytes.len() as u64).to_be_bytes());
        push_string(&mut bytes, &entry.sha256)?;
        bytes.extend_from_slice(&entry.bytes);
    }
    Ok(bytes)
}

pub(crate) fn inventory(plan: &PackagePlan) -> Result<(Vec<u8>, String), DistributionError> {
    #[derive(Serialize)]
    struct Inventory<'a> {
        schema: &'static str,
        context_id: &'a str,
        candidate_id: &'a str,
        plugin_id: &'a str,
        version: &'a str,
        catalog_id: &'a str,
        accepted_inventory_sha256: &'a str,
        source_tree_sha256: &'a str,
        source_date_epoch: u64,
        entries: Vec<InventoryEntry<'a>>,
    }
    #[derive(Serialize)]
    struct InventoryEntry<'a> {
        path: &'a str,
        object_type: &'static str,
        mode: u32,
        sha256: &'a str,
        byte_length: u64,
        role: super::spec::PackageRole,
    }
    let entries = plan
        .entries
        .iter()
        .map(|entry| InventoryEntry {
            path: &entry.path,
            object_type: "regular-file",
            mode: entry.mode,
            sha256: &entry.sha256,
            byte_length: entry.bytes.len() as u64,
            role: entry.role,
        })
        .collect();
    let bytes = serde_json::to_vec(&Inventory {
        schema: "harness-ultragoal.package-inventory.v1",
        context_id: &plan.context_id,
        candidate_id: &plan.candidate_id,
        plugin_id: &plan.plugin_id,
        version: &plan.version,
        catalog_id: &plan.catalog_id,
        accepted_inventory_sha256: &plan.accepted_inventory_sha256,
        source_tree_sha256: &plan.source_tree_sha256,
        source_date_epoch: plan.source_date_epoch,
        entries,
    })
    .map_err(|_| error(DistributionErrorId::InvalidSpec))?;
    let digest = sha256(&bytes);
    Ok((bytes, digest))
}

pub(crate) fn entry_views(entries: &[PackageEntry]) -> Vec<PackageEntry> {
    entries.to_vec()
}

fn push_string(bytes: &mut Vec<u8>, value: &str) -> Result<(), DistributionError> {
    let length = u16::try_from(value.len()).map_err(|_| error(DistributionErrorId::InvalidSpec))?;
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}
