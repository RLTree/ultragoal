//! Source-local production adapter for migration and retirement.
//!
//! This module is crate-private until root wires the sole public migration
//! route. All product effects remain injected and semantically confined.

mod model;
mod registry;
mod runtime;

pub(crate) use model::{
    AdoptedRegistrySnapshot, AuthoritySnapshot, MigrationInputBinding, MigrationInputSource,
    PhysicalBytesPolicy, PlanDisposition, PlannedMigrationEffect, ProductInputSnapshot,
    ProductMigrationError, ProductMigrationPlan, ProductMigrationPlanProjection,
};
pub(crate) use registry::{derive_product_plan, validate_adopted_registry_bytes};
pub(crate) use runtime::{
    apply_product_plan, issue_apply_authorization, recover_product_operation, ApplyAuthorization,
    ApplyAuthorizationAuthority, ApplyOutcome, ApplyOutcomeStatus, AuthorizationRecord,
    ConfinedMigrationEffect, DurableMigrationStore, EffectFault, EffectObservation, JournalPhase,
    MigrationOperation, ReservationRequest, ReservationResult, StoreFault,
};
