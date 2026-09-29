use super::ceiling::ClaimCeiling;
use super::state_authority::{AuthorityRequest, AuthorityRequirement};
use crate::context::EffectClass;
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RepairTargetKind {
    Source,
    Configuration,
    Dependency,
    Capability,
    ExternalDecision,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct RepairTarget {
    pub kind: RepairTargetKind,
    pub id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Repair {
    pub repair_id: String,
    pub target: RepairTarget,
    pub summary: String,
    pub effect: EffectClass,
    pub authority: AuthorityRequirement,
    pub rerun_command_id: String,
    pub authority_decision: Option<AuthorityRequest>,
    pub invalidates_evidence: BTreeSet<String>,
    pub projected_ceiling_after_reverification: Vec<ClaimCeiling>,
}
