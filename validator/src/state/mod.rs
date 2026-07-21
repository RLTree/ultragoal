//! Pure, context-bound product state and deterministic next-action projections.
//!
//! This module accepts authority inputs explicitly. It never reads receipts, generated
//! views, clocks, processes, the network, or the filesystem.

mod adopted;
mod adopted_claims;
mod adopted_registry;
mod catalog;
mod ceiling;
mod engine;
mod finding;
mod findings;
mod identity;
mod limits;
mod next;
mod next_action;
mod normalize;
mod observations;
mod policy;
mod policy_action;
pub(crate) mod policy_authority;
mod policy_repair;
mod product_state;
mod projection;
mod provenance;
mod reduce;
mod repair;
mod routine_observation;
mod snapshot;
mod state_authority;
mod state_error;

#[cfg(test)]
mod tests;

pub(crate) use adopted::{derive_adopted, stage_root_claims};
pub use catalog::{
    ActionDefinition, ActionKind, ActionPriorityClass, CapabilityRequirement, ClaimSpec,
    CommandBinding, DependencyActionCatalog, DependencyActionSpec, DependencyFact,
    DependencyStatus, EvidenceLedActionBinding, FactAuthority, HostGoalObservation, HostGoalStatus,
    InventoryPolicy, RuntimeField, RuntimeMetadata, RuntimeRequirement, RuntimeSource, RuntimeValue,
};
pub use ceiling::{CeilingRelation, ClaimCeiling};
pub use engine::StateEngine;
pub use finding::{CeilingReduction, Finding, FindingSeverity, FindingSource, Scope};
pub use next_action::{NextAction, NextActionKind, NoLegalRoute};
pub use product_state::{ProductGoalState, ProductState};
pub use repair::{Repair, RepairTarget, RepairTargetKind};
pub use routine_observation::{
    RoutineFindingBinding, RoutineFindingObservation, RoutineObservationTransition,
    RoutineObservationWindow,
};
pub use state_authority::{AuthorityRequest, AuthorityRequirement};
pub use state_error::StateError;
