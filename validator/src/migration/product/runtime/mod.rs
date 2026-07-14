use super::super::{digest, valid_identifier, valid_sha256};
pub(crate) use super::model::ApplyAuthorizationAuthority;
use super::model::{
    AuthoritySnapshot, CompatibilityBoundaryObservation, MAX_APPLY_TTL_MS,
    MAX_MACHINE_OUTPUT_BYTES, MigrationInputBinding, MigrationInputSource, PlanDisposition,
    PlannedMigrationEffect, ProductMigrationError, ProductMigrationPlan,
    capture_compatibility_boundary_observation, exact_input_matches,
};
use super::registry::derive_product_plan_from_bound_observation;
use serde::{Deserialize, Serialize};
use std::fmt;

include!("authorization_record.rs");

include!("reservation_request_issue.rs");

include!("compatibility_effect_permit_digest.rs");

include!("migration/reservation.rs");

include!("migration/digest.rs");

include!("rollback_terminal_proof.rs");

include!("apply/outcome_status.rs");

include!("issue_apply_authorization.rs");

include!("apply/plan.rs");

include!("drive_operation.rs");

include!("advance/reserved.rs");

include!("advance/effect_intent.rs");

include!("advance/rollback.rs");

include!("advance/applied.rs");

include!("cas_or_interrupted.rs");

include!("effect_set_digest.rs");
