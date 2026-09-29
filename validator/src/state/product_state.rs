pub use super::finding::{CeilingReduction, Finding, FindingSeverity, FindingSource, Scope};
pub use super::next_action::{NextAction, NextActionKind, NoLegalRoute};
pub use super::repair::{Repair, RepairTarget, RepairTargetKind};
pub use super::routine_observation::{RoutineFindingObservation, RoutineObservationWindow};
pub use super::state_authority::{AuthorityRequest, AuthorityRequirement};
pub use super::state_error::StateError;

use super::catalog::{HostGoalObservation, RuntimeMetadata};
use super::ceiling::ClaimCeiling;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProductGoalState {
    Operating,
    AwaitingAuthority,
    Blocked,
    NoAction,
}

/// Candidate-bound disposition of the observed current behavior. A non-empty
/// finding set is never treated as `no_change`, even when no legal edit exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CurrentBehaviorDisposition {
    NoChange,
    PartialChange,
    ChangeRequired,
    Blocked,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct ProductState {
    pub(crate) schema_version: &'static str,
    pub(crate) state_id: String,
    pub(crate) context_id: String,
    pub(crate) authority_catalog_id: String,
    pub(crate) candidate_id: String,
    pub(crate) dependency_action_catalog_id: String,
    pub(crate) product_goal: ProductGoalState,
    pub(crate) current_behavior: CurrentBehaviorDisposition,
    pub(crate) host_goal: HostGoalObservation,
    pub(crate) runtime_metadata: RuntimeMetadata,
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

    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub fn current_behavior(&self) -> CurrentBehaviorDisposition {
        self.current_behavior
    }

    pub fn product_goal(&self) -> ProductGoalState {
        self.product_goal
    }

    pub fn host_goal(&self) -> &HostGoalObservation {
        &self.host_goal
    }

    pub fn runtime_metadata(&self) -> &RuntimeMetadata {
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
