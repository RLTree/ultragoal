use super::RoutingData;
use crate::inventory::types::{AuthorityState, InventoryEntry, InventoryFinding};
use std::collections::{BTreeMap, BTreeSet};

impl RoutingData {
    pub(crate) fn apply(
        &self,
        entries: &mut BTreeMap<String, InventoryEntry>,
        findings: &mut Vec<InventoryFinding>,
        _duplicate_stable_id_conflicts: &BTreeSet<String>,
        _duplicate_path_conflicts: &BTreeSet<String>,
    ) {
        for entry in entries.values().filter(|entry| {
            entry.authority_state == AuthorityState::Legacy
                && entry.active_status == crate::inventory::types::ActiveStatus::Active
        }) {
            findings.push(InventoryFinding::error(
                "unrouted_legacy_authority",
                Some(&entry.stable_id),
                Some(&entry.relative_path),
                "retired legacy surface has no current authority route".to_owned(),
            ));
        }
    }
}
