use crate::inventory::types::{InventoryEntry, InventoryFinding};
use std::collections::BTreeMap;

pub(crate) const CONTRACT_DIR: &str = "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT";
pub(crate) const MAX_CONTRACT_JSON_BYTES: u64 = 16 * 1024 * 1024;

pub(crate) fn safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

pub(crate) struct RegistryData {
    pub contract_id: String,
    pub counts: BTreeMap<String, usize>,
    pub entries: Vec<InventoryEntry>,
    pub findings: Vec<InventoryFinding>,
    pub legacy_skills: BTreeMap<String, String>,
}
