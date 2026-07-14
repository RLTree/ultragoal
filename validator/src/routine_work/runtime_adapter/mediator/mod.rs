//! Root-grant-consuming routine process mediation.
//!
//! This module verifies but never issues production root authority. It keeps
//! public dispatch, cache persistence, and accepted reuse-witness construction
//! outside this internal execution boundary.

mod filesystem;
mod outcome;
mod process;

use serde::Serialize;
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use crate::context::LiveContext;

use super::execution_authority::{
    PreparedRoutineExecution, RoutineEffectIntent, RoutineEffectRequest, RoutineMediatedIntent,
    RoutineMediationAuthority, RoutineNoOpProjection, RoutineReadSource,
};
use super::{begin_routine_mediation, environment_digest, read_authority_digest};
use crate::routine_work::digest::{canonical, digest_of, framed, sha256, valid};
use crate::routine_work::{
    LocalDirtyTree, RepoPath, RoutineBinding, RoutineError, RoutineErrorId, RoutinePlan,
    trusted_rust_source_execution_observed,
};

use filesystem::{OutputConfinement, PinnedExecutable, ReadConfinement, RootAnchor};
use outcome::{ExecutedArtifact, ResultArtifactWire, ReuseArtifactWire, VerifiedReuseArtifact};
pub(crate) use outcome::{
    RoutineCancellation, RoutineMediationResult, RoutineMediatorStatus, RoutineNodeDisposition,
    RoutineNodeMediation, RoutineReuseInput, RoutineRootGrant,
};
use process::ProcessTermination;

#[path = "grant_scope.rs"]
mod grant_scope;
#[path = "grant_validation.rs"]
mod grant_validation;
#[path = "incomplete_outcome.rs"]
mod incomplete_outcome;
#[path = "intent_mediation.rs"]
mod intent_mediation;
#[path = "no_op_mediation.rs"]
mod no_op_mediation;
#[path = "read_source_binding.rs"]
mod read_source_binding;
#[path = "reuse_input_index.rs"]
mod reuse_input_index;
#[path = "rust_source_observation.rs"]
mod rust_source_observation;

#[cfg(test)]
pub(crate) use filesystem::{test_probe_output_confinement, test_probe_read_confinement};
pub(crate) use grant_scope::*;
pub(crate) use grant_validation::*;
pub(crate) use incomplete_outcome::*;
pub(crate) use intent_mediation::*;
pub(crate) use no_op_mediation::*;
#[cfg(test)]
pub(crate) use process::test_require_root_broker_before_spawn;
pub(crate) use read_source_binding::*;
pub(crate) use reuse_input_index::*;
pub(crate) use rust_source_observation::*;

pub(super) fn validate_routine_program_path(path: &Path) -> Result<(), RoutineError> {
    PinnedExecutable::open_unbound(path).map(|_| ())
}
