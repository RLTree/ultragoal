use super::{PermitTarget, ProductSnapshot};
use crate::orchestration::JournalHead;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResumeRequest {
    pub expected_head: JournalHead,
    pub tick: u64,
    pub live_workers: BTreeSet<String>,
    pub target: PermitTarget,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResumeOutcome {
    pub schema_version: String,
    pub prior_head: JournalHead,
    pub current_head: JournalHead,
    pub root_recovered: bool,
    pub snapshot: ProductSnapshot,
}
