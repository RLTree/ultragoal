#![allow(dead_code, unused_imports)]

mod context {
    pub use ultragoal::context::*;
}

#[path = "../src/cli/capture/mod.rs"]
mod capture;
#[path = "../src/routine_work/mod.rs"]
mod routine_work;
#[path = "routine_work_contract/scenario.rs"]
mod scenario;

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use context::{BuildRequest, LiveContext};
use routine_work::{
    CheckClass, DirtySnapshot, ImpactGraph, LocalDirtyTree, PathMatcher, PlanMode, PlanRequest,
    PreparedRoutineExecution, ProductionRoutineIssuer, RoutineAdapterSpec, RoutineCancellation,
    RoutineInvocationSpec, RoutineMediationResult, RoutineMediatorStatus, RoutineNodeDisposition,
    RoutinePlan, RoutineReuseInput, bind_routine_invocation,
    mediate_prepared_routine_execution_production, plan_routine, prepare_routine_execution,
    set_test_mediator_finish_failure,
};
use scenario::{TempRepo, node, path, route, sha};

#[path = "routine_production_authority_cases/authority_fixture.rs"]
mod authority_fixture;
#[path = "routine_production_authority_cases/authority_redaction.rs"]
mod authority_redaction;
#[path = "routine_production_authority_cases/authority_scenario.rs"]
mod authority_scenario;
#[path = "routine_production_authority_cases/concurrent_reuse_settlement.rs"]
mod concurrent_reuse_settlement;
#[path = "routine_production_authority_cases/effect_capacity_bounds.rs"]
mod effect_capacity_bounds;
#[path = "routine_production_authority_cases/invalid_reuse_recovery.rs"]
mod invalid_reuse_recovery;
#[path = "routine_production_authority_cases/terminal_failure_settlement.rs"]
mod terminal_failure_settlement;

pub(crate) use authority_fixture::*;
pub(crate) use authority_redaction::*;
pub(crate) use authority_scenario::*;
pub(crate) use concurrent_reuse_settlement::*;
pub(crate) use effect_capacity_bounds::*;
pub(crate) use invalid_reuse_recovery::*;
pub(crate) use terminal_failure_settlement::*;
