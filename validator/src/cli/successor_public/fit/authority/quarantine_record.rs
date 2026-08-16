use super::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RepositoryFitAuthorityMigrationRecord {
    pub(crate) schema_version: String,
    pub(crate) status: String,
    pub(crate) authority_status: String,
    pub(crate) generation: u64,
    pub(crate) event_count: usize,
    pub(crate) reservation_count: usize,
    pub(crate) nonterminal_reservation_count: usize,
    pub(crate) pending_envelope_count: usize,
    pub(crate) process_lock_count: usize,
    pub(crate) stored_root: serde_json::Value,
    pub(crate) current_root: serde_json::Value,
    pub(crate) stored_lock: serde_json::Value,
    pub(crate) current_lock: serde_json::Value,
    pub(crate) migration_effect: String,
    pub(crate) migration_authorized: bool,
    pub(crate) quarantine_plan: RepositoryFitAuthorityQuarantineRecord,
    pub(crate) next_action: String,
    pub(crate) claim_effect: String,
    pub(crate) support_limit: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RepositoryFitAuthorityQuarantineRecord {
    pub(crate) schema_version: String,
    pub(crate) plan_id: String,
    pub(crate) status: String,
    pub(crate) source_inventory_sha256: String,
    pub(crate) ledger_sha256: String,
    pub(crate) authority_id: String,
    pub(crate) ledger_head_sha256: String,
    pub(crate) source_owner: String,
    pub(crate) quarantine_owner: String,
    pub(crate) target_format: String,
    pub(crate) strategy: String,
    pub(crate) apply_capability: String,
    pub(crate) operations: Vec<String>,
    pub(crate) rollback: String,
    pub(crate) precondition: String,
    pub(crate) migration_effect: String,
    pub(crate) migration_authorized: bool,
    pub(crate) claim_effect: String,
}

pub(crate) enum QuarantineRecordClassification {
    Other,
    Invalid,
    Quarantine(RepositoryFitAuthorityMigrationRecord),
}

pub(crate) fn classify_quarantine_record(bytes: &[u8]) -> QuarantineRecordClassification {
    if has_layout_whitespace(bytes) {
        return QuarantineRecordClassification::Invalid;
    }
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return QuarantineRecordClassification::Other;
    };
    if value.get("schema_version").and_then(|value| value.as_str())
        != Some("RepositoryFitAuthorityMigrationAdmission-v1")
    {
        return QuarantineRecordClassification::Other;
    }
    match serde_json::from_slice(bytes) {
        Ok(record) => QuarantineRecordClassification::Quarantine(record),
        Err(_) => QuarantineRecordClassification::Invalid,
    }
}

fn has_layout_whitespace(bytes: &[u8]) -> bool {
    let mut in_string = false;
    let mut escaped = false;
    for byte in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                in_string = false;
            }
        } else if *byte == b'"' {
            in_string = true;
        } else if byte.is_ascii_whitespace() {
            return true;
        }
    }
    false
}

pub(crate) fn record_matches_live(
    record: &RepositoryFitAuthorityMigrationRecord,
    live: &RepositoryFitAuthorityMigrationAdmission,
) -> bool {
    record.schema_version == "RepositoryFitAuthorityMigrationAdmission-v1"
        && record.status == live.status
        && record.authority_status == live.authority_status
        && record.generation == live.generation
        && record.event_count == live.event_count
        && record.reservation_count == live.reservation_count
        && record.nonterminal_reservation_count == live.nonterminal_reservation_count
        && record.pending_envelope_count == live.pending_envelope_count
        && record.process_lock_count == live.process_lock_count
        && serde_json::to_value(live.stored_root).ok().as_ref() == Some(&record.stored_root)
        && serde_json::to_value(live.current_root).ok().as_ref() == Some(&record.current_root)
        && serde_json::to_value(live.stored_lock).ok().as_ref() == Some(&record.stored_lock)
        && serde_json::to_value(live.current_lock).ok().as_ref() == Some(&record.current_lock)
        && record.migration_effect == "none"
        && !record.migration_authorized
        && record.claim_effect == "none"
        && record.next_action == live.next_action
        && record.support_limit
            == "read-only authenticated repository-fit authority drift diagnosis and deterministic whole-owner quarantine planning; no migration apply, quarantine effect, bootstrap, installed journey, product, or release claim"
        && live
            .quarantine_plan
            .as_ref()
            .is_some_and(|plan| quarantine_record_matches(&record.quarantine_plan, plan))
}

pub(crate) fn quarantine_record_matches(
    record: &RepositoryFitAuthorityQuarantineRecord,
    plan: &RepositoryFitAuthorityQuarantinePlan,
) -> bool {
    record.schema_version == "RepositoryFitAuthorityQuarantinePlan-v1"
        && record.plan_id == plan.plan_id
        && record.status == "review_required"
        && record.source_inventory_sha256 == plan.source_inventory_sha256
        && record.ledger_sha256 == plan.ledger_sha256
        && record.authority_id == plan.authority_id
        && record.ledger_head_sha256 == plan.ledger_head_sha256
        && record.source_owner == plan.source_owner
        && record.quarantine_owner == plan.quarantine_owner
        && record.target_format == plan.target_format
        && record.strategy == plan.strategy
        && record.apply_capability == plan.apply_capability
        && record.operations
            == plan
                .operations
                .iter()
                .map(|operation| (*operation).to_owned())
                .collect::<Vec<_>>()
        && record.rollback == plan.rollback
        && record.precondition
            == "same authenticated complete source inventory under exclusive parent custody; quarantine destination absent"
        && record.migration_effect == "none"
        && !record.migration_authorized
        && record.claim_effect == "none"
}
