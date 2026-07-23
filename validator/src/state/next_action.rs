use super::catalog::ActionDefinition;
use super::repair::RepairTarget;
use super::state_authority::{AuthorityRequest, AuthorityRequirement};
use crate::context::EffectClass;
use serde::Serialize;
use std::collections::BTreeMap;

use super::catalog::ActionPriorityClass;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NextActionKind {
    Command,
    AuthorityRequest,
    NoLegalRoute,
    NoOp,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NoLegalRoute {
    pub target: RepairTarget,
    pub required_authority: AuthorityRequirement,
    pub required_decision: Option<AuthorityRequest>,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NextAction {
    pub kind: NextActionKind,
    pub action_id: String,
    pub priority: u32,
    pub repair_id: Option<String>,
    pub effect: EffectClass,
    pub authority: AuthorityRequirement,
    pub command_id: Option<String>,
    pub exact_command: Option<Vec<String>>,
    pub authority_request: Option<AuthorityRequest>,
    pub no_legal_route: Option<NoLegalRoute>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_class: Option<ActionPriorityClass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_transition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brief_digest: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub active_trigger_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parked_trigger_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_mode: Option<crate::engineering_advisory::VerificationModeContract>,
    pub selection_rule: &'static str,
}

pub(crate) fn attach_verification(
    mut next: NextAction,
    action: &ActionDefinition,
    verification_modes: &BTreeMap<String, crate::engineering_advisory::VerificationModeContract>,
) -> NextAction {
    next.verification_mode = verification_modes.get(&action.action_id).cloned();
    next
}
