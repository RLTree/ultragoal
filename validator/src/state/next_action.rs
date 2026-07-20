use super::repair::RepairTarget;
use super::state_authority::{AuthorityRequest, AuthorityRequirement};
use crate::context::EffectClass;
use serde::Serialize;

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
    pub selection_rule: &'static str,
}
