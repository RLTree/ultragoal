use super::scenario::{Fixture, assert_zero_write, effects_for, execute, git, git_status, snapshot};
use crate::repository_fit::product_adapter::catalog::CANONICAL_TEMPLATES;
use crate::repository_fit::product_adapter::{inspect_target, plan_target, verify_target};
use crate::repository_fit::{FitErrorId, RepositoryClass};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

#[path = "fresh_inspect_and_plan_are_deterministic_zero_write.rs"]
mod fresh_inspect_and_plan_are_deterministic_zero_write;
#[path = "mode_drift_invalidates_the_mode_bound_adapter_verification_proof.rs"]
mod mode_drift_invalidates_the_mode_bound_adapter_verification_proof;
#[path = "local_state_policy.rs"]
mod local_state_policy;
