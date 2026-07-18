//! Source-local production adapter for migration and retirement.
//!
//! This module is crate-private until root wires the sole public migration
//! route. All product effects remain injected and semantically confined.

mod host;
mod model;
mod registry;
mod runtime;

#[cfg(test)]
pub(crate) use host::provision_darwin_migration_host_for_test;
#[cfg(test)]
pub(crate) use host::{DarwinMigrationAdapters, DarwinMigrationHost, HostError};
#[cfg(test)]
pub(crate) use model::ProductMigrationPlanProjection;
pub(crate) use model::{
    AdoptedRegistrySnapshot, AuthoritySnapshot, MigrationInputBinding, MigrationInputSource,
    PlanDisposition, PlannedMigrationEffect, ProductInputSnapshot, ProductMigrationError,
    ProductMigrationPlan,
};
#[cfg(test)]
pub(crate) use registry::{derive_product_plan, validate_adopted_registry_bytes};
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
