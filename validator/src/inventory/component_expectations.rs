use super::registry::RegistryData;
use super::types::{InventoryEntry, InventoryFinding};
use std::collections::BTreeSet;

pub(crate) struct DiscoveryData {
    pub entries: Vec<InventoryEntry>,
    pub findings: Vec<InventoryFinding>,
}

pub(crate) fn expected_names(registry: &RegistryData, prefix: &str) -> BTreeSet<String> {
    registry
        .entries
        .iter()
        .filter_map(|entry| entry.stable_id.strip_prefix(prefix))
        .map(ToOwned::to_owned)
        .collect()
}
