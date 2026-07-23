//! Source-local production adapter for migration and retirement.
//!
//! This module is crate-private until root wires the sole public migration
//! route. All product effects remain injected and semantically confined.

mod activation;
mod activation_inputs;
#[cfg(test)]
mod host;
mod model;
mod registry;
#[cfg(test)]
mod runtime;

pub(crate) use activation::{SkillFamilyActivationProjection, observe_skill_family_activation};
#[cfg(test)]
pub(crate) use host::provision_darwin_migration_host_for_test;
#[cfg(test)]
pub(crate) use host::{DarwinMigrationAdapters, DarwinMigrationHost, HostError};
pub(crate) use model::{
    AdoptedRegistrySnapshot, ProductInputSnapshot, ProductMigrationError,
    ProductMigrationPlanProjection,
};
#[cfg(test)]
pub(crate) use model::{
    AuthoritySnapshot, MigrationInputBinding, MigrationInputSource, PlanDisposition,
    PlannedMigrationEffect, ProductMigrationPlan,
};
#[cfg(test)]
pub(crate) use registry::derive_product_plan;
pub(crate) use registry::derive_read_only_product_plan;
#[cfg(test)]
pub(crate) use registry::validate_adopted_registry_bytes;
#[cfg(test)]
pub(crate) use runtime::{
    ApplyAuthorizationAuthority, AuthorizationRecord, ConfinedMigrationEffect,
    DurableMigrationStore, EffectFault, EffectObservation, MigrationOperation, ReservationRequest,
    ReservationResult, StoreFault,
};
#[cfg(test)]
pub(crate) use runtime::{
    ApplyOutcomeStatus, JournalPhase, apply_product_plan, issue_apply_authorization,
    recover_product_operation,
};
