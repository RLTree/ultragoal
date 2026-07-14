use super::PermitTarget;
use crate::orchestration::{EffectResolution, JournalHead};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconcileRequest {
    pub expected_head: JournalHead,
    pub tick: u64,
    pub live_workers: BTreeSet<String>,
    pub lease_id: String,
    pub resolution: EffectResolution,
    pub target: PermitTarget,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconcileOutcome {
    pub schema_version: String,
    pub prior_head: JournalHead,
    pub current_head: JournalHead,
    pub settled_operation_id: String,
    pub snapshot_id: String,
}
