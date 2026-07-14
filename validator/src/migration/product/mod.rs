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
pub(crate) use host::{
    DarwinApplyAuthorizationAuthority, DarwinConfinedMigrationEffect, DarwinDurableMigrationStore,
    DarwinMigrationAdapters, DarwinMigrationHost, DarwinMigrationInputSource, HostError,
};
pub(crate) use model::{
    AdoptedRegistrySnapshot, AuthoritySnapshot, MigrationInputBinding, MigrationInputSource,
    PhysicalBytesPolicy, PlanDisposition, PlannedMigrationEffect, ProductInputSnapshot,
    ProductMigrationError, ProductMigrationPlan, ProductMigrationPlanProjection,
};
pub(crate) use registry::{derive_product_plan, validate_adopted_registry_bytes};
pub(crate) use runtime::{
    ApplyAuthorization, ApplyAuthorizationAuthority, ApplyOutcome, ApplyOutcomeStatus,
    AuthorizationRecord, ConfinedMigrationEffect, DurableMigrationStore, EffectFault,
    EffectObservation, JournalPhase, MigrationOperation, ReservationRequest, ReservationResult,
    StoreFault, apply_product_plan, issue_apply_authorization, recover_product_operation,
};
