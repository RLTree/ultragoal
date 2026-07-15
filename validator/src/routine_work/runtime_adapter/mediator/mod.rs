//! Root-grant-consuming routine process mediation.
//!
//! This module verifies but never issues production root authority. It keeps
//! public dispatch, cache persistence, and accepted reuse-witness construction
//! outside this internal execution boundary.

mod filesystem;
mod outcome;
mod process;

use serde::Serialize;
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
    CleanupEvidence, FailureEvidence, LocalDirtyTree, PanicEvidence, ProcessCustodyEvidence,
    RESERVATION_FAILURE_SCHEMA, RepoPath, ReservationFailureDisposition,
    ReservationFailureEvidence, RoutineBinding, RoutineError, RoutineErrorId, RoutinePlan,
    transition_failure_error, trusted_rust_source_execution_observed,
};

pub(crate) use filesystem::{
    ObjectIdentity, OutputConfinement, PinnedExecutable, ReadConfinement, RootAnchor, StagedProgram,
};
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
#[cfg(test)]
#[path = "reservation_lifecycle_tests.rs"]
mod reservation_lifecycle_tests;
#[path = "reservation_state/mod.rs"]
mod reservation_state;
#[cfg(test)]
#[path = "reservation_unwind_tests.rs"]
mod reservation_unwind_tests;
#[path = "reuse_input_index.rs"]
mod reuse_input_index;
#[path = "rust_source_observation.rs"]
mod rust_source_observation;
#[cfg(test)]
#[path = "terminal_settlement_fixture.rs"]
mod terminal_settlement_fixture;
#[cfg(test)]
#[path = "terminal_settlement_tests.rs"]
mod terminal_settlement_tests;

#[cfg(test)]
pub(crate) use filesystem::{
    validate_output_confinement_after, validate_read_confinement_after_bind,
};
pub(crate) use grant_scope::*;
pub(crate) use grant_validation::*;
pub(crate) use incomplete_outcome::*;
use intent_mediation::mediate_intent;
pub(super) use no_op_mediation::*;
pub(super) use read_source_binding::mediate_prepared_routine_execution;
pub(crate) use read_source_binding::{
    bind_read_sources, grant_identity, grant_seal, registry, validate_read_sources,
};
use reservation_state::{
    AttemptReservation, observe_staged_transition, reserve_grant, run_reserved,
};
pub(crate) use reuse_input_index::*;
pub(crate) use rust_source_observation::*;

pub(super) fn validate_routine_program_path(path: &Path) -> Result<(), RoutineError> {
    PinnedExecutable::open_unbound(path).map(|_| ())
}
