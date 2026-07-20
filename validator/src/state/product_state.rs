use super::ceiling::ClaimCeiling;
use crate::context::EffectClass;
use serde::Serialize;
use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FindingSeverity {
    Blocked,
    Error,
    Warning,
    Info,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthorityRequirement {
    None,
    Workspace,
    Root,
    External,
    HumanDestructive,
}

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
pub struct CeilingReduction {
    pub claim_id: String,
    pub dimensions: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AuthorityRequest {
    pub target: String,
    pub consequence: String,
    pub reversible: bool,
    pub accepted_loss_required: Option<String>,
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

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Scope {
    pub surface: String,
    pub relative_path: Option<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum FindingSource {
    LiveContext {
        context_id: String,
    },
    AuthorityCatalog {
        catalog_id: String,
        code: String,
    },
    DependencyCatalog {
        catalog_id: String,
        observation_id: String,
    },
    RuntimeMetadata {
        field: String,
    },
    StatePolicy {
        catalog_id: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Finding {
    pub finding_id: String,
    pub code: String,
    pub severity: FindingSeverity,
    pub source: FindingSource,
    pub scope: Scope,
    pub dependency_ids: BTreeSet<String>,
    pub cause: String,
    pub effect: EffectClass,
    pub authority: AuthorityRequirement,
    pub repair: Repair,
    pub affected_claims: BTreeSet<String>,
    pub ceiling_reductions: Vec<CeilingReduction>,
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProductGoalState {
    Operating,
    AwaitingAuthority,
    Blocked,
    NoAction,
}

/// Diagnostic evidence associated with a terminal routine event.
///
/// This is intentionally outside `findings`, `repairs`, and claim ceilings:
/// the event may help an operator explain interruption, recovery, or reuse,
/// but it cannot create or settle a product finding.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RoutineFindingObservation {
    pub event_id: String,
    pub continuation_id: String,
    pub terminal_ledger_head: String,
    pub transition: RoutineObservationTransition,
    pub outcome: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RoutineObservationTransition {
    Interrupted,
    Recovered,
    Reused,
    Executed,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RoutineObservationWindow {
    NotQueried,
    Absent,
    Available,
    Saturated,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProductState {
    pub(crate) schema_version: &'static str,
    pub(crate) state_id: String,
    pub(crate) context_id: String,
    pub(crate) authority_catalog_id: String,
    pub(crate) dependency_action_catalog_id: String,
    pub(crate) product_goal: ProductGoalState,
    pub(crate) host_goal: super::catalog::HostGoalObservation,
    pub(crate) runtime_metadata: super::catalog::RuntimeMetadata,
    pub(crate) findings: Vec<Finding>,
    pub(crate) repairs: Vec<Repair>,
    pub(crate) claim_ceilings: Vec<ClaimCeiling>,
    pub(crate) next_action: NextAction,
    pub(crate) routine_observations: Vec<RoutineFindingObservation>,
    pub(crate) routine_observation_window: RoutineObservationWindow,
}

impl ProductState {
    pub fn state_id(&self) -> &str {
        &self.state_id
    }
    pub fn context_id(&self) -> &str {
        &self.context_id
    }
    pub fn authority_catalog_id(&self) -> &str {
        &self.authority_catalog_id
    }
    pub fn product_goal(&self) -> ProductGoalState {
        self.product_goal
    }
    pub fn host_goal(&self) -> &super::catalog::HostGoalObservation {
        &self.host_goal
    }
    pub fn runtime_metadata(&self) -> &super::catalog::RuntimeMetadata {
        &self.runtime_metadata
    }
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }
    pub fn repairs(&self) -> &[Repair] {
        &self.repairs
    }
    pub fn claim_ceilings(&self) -> &[ClaimCeiling] {
        &self.claim_ceilings
    }
    pub fn next_action(&self) -> &NextAction {
        &self.next_action
    }
    pub fn routine_observations(&self) -> &[RoutineFindingObservation] {
        &self.routine_observations
    }
    pub fn routine_observation_window(&self) -> RoutineObservationWindow {
        self.routine_observation_window
    }

    /// The public observation adapter is the only caller. Keeping this
    /// crate-visible prevents event data from becoming a state-derivation
    /// input or a finding mutation API.
    pub(crate) fn attach_routine_observations(
        &mut self,
        observations: Vec<RoutineFindingObservation>,
        window: RoutineObservationWindow,
    ) {
        self.routine_observations = observations;
        self.routine_observation_window = window;
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum StateError {
    InvalidCatalog(String),
    ResourceLimit(String),
    Serialization(String),
    StaleContext(String),
}

impl fmt::Display for StateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCatalog(message) => write!(formatter, "invalid state catalog: {message}"),
            Self::ResourceLimit(message) => write!(formatter, "state resource limit: {message}"),
            Self::Serialization(message) => {
                write!(formatter, "state serialization failed: {message}")
            }
            Self::StaleContext(message) => write!(formatter, "live context is stale: {message}"),
        }
    }
}

impl std::error::Error for StateError {}
