//! Public `check routine` production adapter.
//!
//! The repository supplies only a strict affected graph and exact source
//! bindings. Dirty work is limited to the closed `rust-source-syntax-v1`
//! behavior executed by the pinned current `ultragoal` program over
//! mediator-held framed bytes; repository content cannot select a program,
//! argument template, fallback, or child-authored outcome.

mod behavior_child;
mod host;
mod manifest;
mod outcome;

use self::host::{CacheBinding, HostState};
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
    ImpactGraph, LocalDirtyTree, PlanRequest, PreparedRoutineExecution, ProductionRoutineIssuer,
    RepoPath, RoutineAdapterSpec, RoutineArtifactPublisher, RoutineCancellation,
    RoutineInvocationSpec, RoutineMediatorStatus, RoutinePlan, RoutineReuseInput,
    RunnerObservation, SelectedRoutineNode, TransitiveInputExpectation,
    bind_rust_source_syntax_invocation, load_production_catalog,
    mediate_prepared_routine_execution_production, plan_routine, prepare_routine_execution,
    validate_immutable_routine_program,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[path = "invocation_binding.rs"]
mod invocation_binding;
#[path = "source_configuration.rs"]
mod source_configuration;
#[path = "source_selection.rs"]
mod source_selection;

pub(crate) use invocation_binding::*;
pub(crate) use source_configuration::*;
pub(crate) use source_selection::*;
