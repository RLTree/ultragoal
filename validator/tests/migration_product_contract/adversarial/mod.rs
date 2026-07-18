use super::runtime_fixtures::*;
use crate::migration::product::{
    AdoptedRegistrySnapshot, ApplyOutcomeStatus, ProductInputSnapshot,
    ProductMigrationPlanProjection, apply_product_plan, derive_product_plan,
    issue_apply_authorization, recover_product_operation,
};
use crate::migration::{
    InventorySurface, InventorySurfaceObservation, MigrationInventory, SurfaceFileKind,
    SurfaceStatus,
};
use serde_json::json;
use std::sync::{Arc, Barrier};
use std::thread;

include!("fixture_catalog_names_the_positive_security_recovery_and_false_pass_envelope.rs");

include!("duplicate_reader_writer_public_and_generated_authority_is_rejected.rs");

include!("every/compatibility_prerequisite_is_mandatory_and_malformed_values_fail_closed.rs");

include!("authorization_rejects_crossed_boundary_source_substitution_and_version_rollback.rs");

include!("crossed_compatibility_boundary_rolls_back_prior_retirement_effect.rs");

include!(
    "every/prerequisite_substitution_changes_plan_effect_and_sealed_authorization_identity.rs"
);

include!(
    "stale_registry_candidate_session_plan_projection_and_authority_rebinding_are_rejected.rs"
);

include!("filesystem_alias_special_hardlink_traversal_and_unicode_inputs_are_rejected.rs");
