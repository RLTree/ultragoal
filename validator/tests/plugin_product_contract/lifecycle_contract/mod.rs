use super::plugin_product::journey_matrix::{JOURNEYS, validate_journey_matrix};
use super::plugin_product::lifecycle::{
    ApplyDisposition, LifecycleAuthorization, LifecycleEffect, LifecycleEffectAdapter,
    LifecycleError, LifecycleIntent, LifecyclePlan, LifecycleRequest, LifecycleState,
    PackageAuthority, RecoveryToken, Version, apply, plan, recover, recovery_token, verify,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};

include!("lifecycle_fixtures.rs");

include!("zero_write_trap_adapter.rs");

include!("explicit_failed_update_recovery_is_prior_bound.rs");

include!("every_read_only_effect_failure_is_causal_and_closes_without_recovery_calls.rs");

include!("plan_identity_binds_authorization_and_rejects_post_plan_mutation.rs");

include!("serialized_plan_is_transport_only_and_cannot_recover_apply_authority.rs");

include!("serialized_recovery_token_cannot_recover_restore_authority.rs");

include!("recovery_during_apply_refuses_until_effects_finish_and_then_arms.rs");
