//! Public `check routine` production adapter.
//!
//! The repository supplies a strict affected graph and immutable catalog. It
//! never supplies arbitrary shell text: each selected recipe must equal one
//! adapter-generated pass/fail template before it can enter the accepted
//! production ledger and mediator.

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
    RepoPath, RoutineAdapterSpec, RoutineCancellation, RoutineInvocationSpec,
    RoutineMediatorStatus, RoutinePlan, RoutineReuseInput, RunnerObservation, SelectedRoutineNode,
    TransitiveInputExpectation, bind_routine_invocation_with_environment_and_read_sources,
    load_production_catalog, mediate_prepared_routine_execution_production, plan_routine,
    prepare_routine_execution,
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
