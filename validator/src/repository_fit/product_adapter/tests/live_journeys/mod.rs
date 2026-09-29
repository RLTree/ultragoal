#![cfg(target_vendor = "apple")]

use super::super::catalog::CANONICAL_TEMPLATES;
use super::super::protocol::OpaqueFitApplyRequest;
use super::super::root_permit::{
    RepositoryFitApplyFailure, RepositoryFitApplyOutcome, RepositoryFitPermitEffects,
    TestRepositoryFitPermitAuthority, apply_with_root_permit,
    before_final_green_observation_for_test, duplicate_authorization_for_test,
    permit_seal_stage_for_test,
};
use super::super::{
    AdapterErrorId, inspect_target, plan_target, prepare_apply_request, verify_target,
};
use super::scenario::{git_status, snapshot};
use crate::context::{BuildRequest, LiveContext};
use crate::repository_fit::local::LocalEffects;
use crate::repository_fit::{
    CanonicalPath, ExpectedContent, FitEffects, FitError, FitReader, digest,
};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

#[path = "positive_supported_host_fresh_setup_and_repeat_use_are_exact_and_idempotent.rs"]
mod positive_supported_host_fresh_setup_and_repeat_use_are_exact_and_idempotent;
#[path = "race_after_effect_is_ambiguous_and_fresh_replanning_recovers.rs"]
mod race_after_effect_is_ambiguous_and_fresh_replanning_recovers;
#[path = "scenario_fixture.rs"]
mod scenario_fixture;

pub(crate) use scenario_fixture::*;
