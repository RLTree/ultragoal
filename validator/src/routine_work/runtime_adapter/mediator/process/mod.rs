use sha2::{Digest, Sha256};
#[cfg(test)]
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::routine_work::{
    CleanupEvidence, FailureEvidence, PanicEvidence, ProcessCustodyEvidence, RoutineError,
    RoutineErrorId,
};

use super::filesystem::{OutputConfinement, PinnedExecutable, RootAnchor};
use super::outcome::RoutineCancellation;

#[path = "custody_settlement.rs"]
mod custody_settlement;
#[cfg(target_os = "macos")]
#[path = "darwin_hooks.rs"]
mod darwin_hooks;
#[cfg(target_os = "macos")]
#[path = "darwin_suspended_launch.rs"]
mod darwin_suspended_launch;
#[path = "execution.rs"]
mod execution;
#[path = "failure_injection.rs"]
mod failure_injection;
#[cfg(all(target_os = "macos", test))]
#[path = "group_custody.rs"]
mod group_custody;
#[cfg(target_os = "macos")]
#[path = "object_bound_launch.rs"]
mod object_bound_launch;
#[path = "observation/digest.rs"]
mod observation_digest;
#[path = "observation/failure.rs"]
mod observation_failure;
#[cfg(target_os = "macos")]
#[path = "sandbox_profile.rs"]
mod sandbox_profile;
#[path = "spawn_test_observation.rs"]
mod spawn_test_observation;

#[cfg(test)]
type ProcessHook = Box<dyn FnOnce() + Send + 'static>;

#[cfg(test)]
thread_local! {
    static PRE_SPAWN_HOOK: RefCell<Option<ProcessHook>> = RefCell::new(None);
    static POST_SPAWN_HOOK: RefCell<Option<ProcessHook>> = RefCell::new(None);
    static LOADED_OBJECT_HOOK: RefCell<Option<ProcessHook>> = RefCell::new(None);
}

#[cfg(test)]
pub(crate) fn set_test_process_pre_spawn_hook(hook: impl FnOnce() + Send + 'static) {
    PRE_SPAWN_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(test)]
pub(crate) fn set_test_process_post_spawn_hook(hook: impl FnOnce() + Send + 'static) {
    POST_SPAWN_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(test)]
pub(crate) fn set_test_loaded_object_hook(hook: impl FnOnce() + Send + 'static) {
    LOADED_OBJECT_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(all(test, target_os = "macos"))]
pub(crate) fn test_last_spawn_group_absent() -> Result<bool, RoutineError> {
    match test_last_spawn_group() {
        Some(group) => process_group_exists(group).map(|exists| !exists),
        None => Ok(false),
    }
}

#[cfg(test)]
fn run_test_process_pre_spawn_hook() {
    if let Some(hook) = PRE_SPAWN_HOOK.with(|slot| slot.borrow_mut().take()) {
        hook();
    }
}

#[cfg(test)]
fn run_test_process_post_spawn_hook() {
    if let Some(hook) = POST_SPAWN_HOOK.with(|slot| slot.borrow_mut().take()) {
        hook();
    }
}

#[cfg(test)]
fn run_test_loaded_object_hook() {
    if let Some(hook) = LOADED_OBJECT_HOOK.with(|slot| slot.borrow_mut().take()) {
        hook();
    }
}

#[cfg(not(test))]
fn run_test_process_pre_spawn_hook() {}

#[cfg(not(test))]
fn run_test_process_post_spawn_hook() {}

#[cfg(not(test))]
fn run_test_loaded_object_hook() {}

pub(in crate::routine_work) use custody_settlement::ObservedProcessCustody;
pub(crate) use custody_settlement::take_process_custody_panic;
#[cfg(target_os = "macos")]
pub(crate) use darwin_suspended_launch::*;
pub(crate) use execution::*;
pub(crate) use failure_injection::*;
#[cfg(all(target_os = "macos", test))]
pub(crate) use group_custody::*;
#[cfg(target_os = "macos")]
pub(crate) use object_bound_launch::*;
pub(crate) use observation_digest::*;
pub(crate) use observation_failure::*;
#[cfg(target_os = "macos")]
pub(crate) use sandbox_profile::*;
pub(crate) use spawn_test_observation::*;

#[cfg(all(test, target_os = "macos"))]
#[path = "custody_transition_tests.rs"]
mod custody_transition_tests;
#[cfg(test)]
#[path = "object_binding_tests.rs"]
mod object_binding_tests;
#[cfg(test)]
#[path = "revalidation_tests.rs"]
mod revalidation_tests;
#[cfg(test)]
#[path = "termination_tests.rs"]
mod termination_tests;
