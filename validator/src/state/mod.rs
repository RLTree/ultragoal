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
mod findings;
mod identity;
mod limits;
mod next;
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
mod snapshot;

#[cfg(test)]
mod tests;

pub(crate) use adopted::{derive_adopted, stage_root_claims};
pub use catalog::{
    ActionDefinition, ActionKind, CapabilityRequirement, ClaimSpec, CommandBinding,
    DependencyActionCatalog, DependencyActionSpec, DependencyFact, DependencyStatus, FactAuthority,
    HostGoalObservation, HostGoalStatus, InventoryPolicy, RuntimeField, RuntimeMetadata,
    RuntimeRequirement, RuntimeSource, RuntimeValue,
};
pub use ceiling::{CeilingRelation, ClaimCeiling};
pub use engine::StateEngine;
pub use product_state::{
    AuthorityRequest, AuthorityRequirement, CeilingReduction, Finding, FindingSeverity,
    FindingSource, NextAction, NextActionKind, NoLegalRoute, ProductGoalState, ProductState,
    Repair, RepairTarget, RepairTargetKind, RoutineFindingBinding, RoutineFindingObservation,
    RoutineObservationTransition, RoutineObservationWindow, Scope, StateError,
};
