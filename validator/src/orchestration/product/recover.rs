use super::{PermitTarget, ProductSnapshot};
use crate::orchestration::{Binding, JournalHead};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoverRequest {
    pub expected_prior_head: JournalHead,
    pub expected_event_id: String,
    pub recovered_binding: Binding,
    pub tick: u64,
    pub live_workers: BTreeSet<String>,
    pub target: PermitTarget,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoverOutcome {
    pub schema_version: String,
    pub workspace_identity: String,
    pub workspace_path_current_after_commit: bool,
    pub prior_head: JournalHead,
    pub recovered_head: JournalHead,
    pub snapshot: ProductSnapshot,
}
