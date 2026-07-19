use super::super::{
    InventorySurface, SurfaceFileKind, SurfaceStatus, digest, safe_reference, valid_identifier,
    valid_sha256, valid_stable_identifier,
};
use super::model::{
    AdoptedAuthorityPostcondition, ApplyAuthorizationAuthority, AuthoritySnapshot,
    CompatibilityBoundaryObservation, CompatibilityPrerequisites, MigrationInputBinding,
    PlanDisposition, PlannedMigrationEffect, PlannedMigrationEffectDefinition,
    ProductInputSnapshot, ProductMigrationError, ProductMigrationPlan, ProductPlanItem,
    ProductPlanItemDefinition, REQUIRED_FALSE_PASS_CONTROLS,
    capture_compatibility_boundary_observation,
};
use crate::inventory::MAX_MIGRATION_REGISTRY_BYTES;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

include!("schema.rs");

include!("derive_product_plan_parts.rs");

include!("match_validation.rs");

include!("validate_adoption.rs");
