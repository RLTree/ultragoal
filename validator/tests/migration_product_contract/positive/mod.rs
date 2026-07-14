use super::runtime_fixtures::*;
use crate::migration::SurfaceStatus;
use crate::migration::product::{
    ApplyOutcomeStatus, PlanDisposition, apply_product_plan, derive_product_plan,
    issue_apply_authorization, validate_adopted_registry_bytes,
};

include!("live_adopted_registry_bytes_match_the_supported_parser_contract.rs");

include!("explicitly_adopted_compatibility_transition_applies_once_and_repeats_idempotently.rs");
