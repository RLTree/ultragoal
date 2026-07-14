use std::collections::BTreeSet;
use std::fs;
use std::net::TcpListener;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use super::context::{BuildRequest, LiveContext};
use super::routine_work::{
    CheckClass, DirtySnapshot, ImpactGraph, LocalDirtyTree, PathMatcher, PlanMode, PlanRequest,
    PreparedRoutineExecution, RoutineAdapterSpec, RoutineCancellation, RoutineEffectRequest,
    RoutineInvocationSpec, RoutineMediationResult, RoutineMediatorStatus, RoutineNodeDisposition,
    RoutinePlan, RoutineReuseInput, RoutineRootGrant, TestProcessSetupFailure,
    bind_routine_invocation, bind_routine_invocation_with_read_sources,
    mediate_prepared_routine_execution, plan_routine, prepare_routine_execution,
    set_test_mediator_finish_failure, set_test_mediator_post_spawn_hook,
    set_test_mediator_pre_spawn_hook, set_test_output_capture_hook, set_test_process_setup_failure,
    set_test_read_source_capture_hook, test_spawn_count,
};
use super::scenario::{TempRepo, node, path, route};

#[path = "runtime_mediator_cases/cancellation_reaping.rs"]
mod cancellation_reaping;
#[path = "runtime_mediator_cases/containment_adversary.rs"]
mod containment_adversary;
#[path = "runtime_mediator_cases/context_mutation_recovery.rs"]
mod context_mutation_recovery;
#[path = "runtime_mediator_cases/executable_substitution_rejection.rs"]
mod executable_substitution_rejection;
#[path = "runtime_mediator_cases/fork_bypass_reaping.rs"]
mod fork_bypass_reaping;
#[path = "runtime_mediator_cases/invocation_fixture.rs"]
mod invocation_fixture;
#[path = "runtime_mediator_cases/mapping_adversary.rs"]
mod mapping_adversary;
#[path = "runtime_mediator_cases/mediator_scenario.rs"]
mod mediator_scenario;
#[path = "runtime_mediator_cases/read_source_refusals.rs"]
mod read_source_refusals;
#[path = "runtime_mediator_cases/reuse_integrity.rs"]
mod reuse_integrity;
#[path = "runtime_mediator_cases/script_replacement_rejection.rs"]
mod script_replacement_rejection;
#[path = "runtime_mediator_cases/setup_failure_recovery.rs"]
mod setup_failure_recovery;
#[path = "runtime_mediator_cases/system_shell_eligibility.rs"]
mod system_shell_eligibility;
#[path = "runtime_mediator_cases/unsafe_output_rejection.rs"]
mod unsafe_output_rejection;

pub(crate) use cancellation_reaping::*;
pub(crate) use containment_adversary::*;
pub(crate) use context_mutation_recovery::*;
pub(crate) use executable_substitution_rejection::*;
pub(crate) use fork_bypass_reaping::*;
pub(crate) use invocation_fixture::*;
pub(crate) use mapping_adversary::*;
pub(crate) use mediator_scenario::*;
pub(crate) use read_source_refusals::*;
pub(crate) use reuse_integrity::*;
pub(crate) use script_replacement_rejection::*;
pub(crate) use setup_failure_recovery::*;
pub(crate) use system_shell_eligibility::*;
pub(crate) use unsafe_output_rejection::*;
