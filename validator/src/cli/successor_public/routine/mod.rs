//! Public `check routine` production adapter.
//!
//! The repository supplies only a strict affected graph and exact source
//! bindings. The canonical adapter owns planning, exact local issuance, and
//! parent-authenticated observations; repository content cannot select a
//! program, argument template, fallback, or child-authored outcome.

mod behavior_child;
mod host;
mod manifest;
mod outcome;

use self::host::HostState;
use self::manifest::{LoadedManifest, MANIFEST_PATH};
use self::outcome::PublicFailure;
use crate::cli::successor::runtime::RuntimeOutcome;
use crate::cli::successor::{
    CheckProfile, EffectClass, OptionName, ParsedInvocation, ParsedValue, SuccessorCommand,
};
use crate::context::{BuildRequest, LiveContext, ToolCapability};
use crate::inventory::{ADOPTED_HANDOFF_DIGEST_CONFIG_KEY, ADOPTED_HANDOFF_MANIFEST_SHA256};
use crate::routine_work::{
    AdoptedRoutineNode, BoundCatalogInvocation, CatalogAdoption, CatalogSelectionRequest,
    ImpactGraph, LocalDirtyTree, PlanRequest, PreparedRoutineExecution, RepoPath,
    RoutineAdapterSpec, RoutineCancellation, RoutineInvocationSpec, RoutinePlan, RoutineReuseInput,
    RunnerObservation, SelectedRoutineNode, TransitiveInputExpectation,
    bind_rust_source_syntax_invocation, load_production_catalog, mediate_public_routine_execution,
    plan_routine, prepare_routine_execution, validate_immutable_routine_program,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[path = "invocation_binding.rs"]
mod invocation_binding;
#[path = "source/configuration.rs"]
mod source_configuration;
#[path = "source/context.rs"]
mod source_context;
#[path = "source/selection.rs"]
mod source_selection;

pub(crate) use invocation_binding::*;
pub(crate) use source_configuration::*;
pub(crate) use source_selection::*;
