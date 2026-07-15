use sha2::{Digest, Sha256};
#[cfg(test)]
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
#[cfg(test)]
use std::sync::{Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

use crate::routine_work::{RoutineError, RoutineErrorId};

use super::filesystem::{OutputConfinement, PinnedExecutable, ReadConfinement, RootAnchor};
use super::outcome::RoutineCancellation;

#[cfg(target_os = "macos")]
#[path = "darwin_child_custody.rs"]
mod darwin_child_custody;
#[cfg(target_os = "macos")]
#[path = "darwin_suspended_launch.rs"]
mod darwin_suspended_launch;
#[cfg(target_os = "macos")]
#[path = "loaded_executable_identity.rs"]
mod loaded_executable_identity;
#[cfg(target_os = "macos")]
#[path = "object_bound_launch.rs"]
mod object_bound_launch;
#[path = "process_execution.rs"]
mod process_execution;
#[path = "process_group_observation.rs"]
mod process_group_observation;
#[path = "process_input_write.rs"]
mod process_input_write;
#[path = "process_output_drain.rs"]
mod process_output_drain;
#[path = "spawn_test_observation.rs"]
mod spawn_test_observation;

#[cfg(test)]
type ProcessHook = Box<dyn FnOnce() + Send + 'static>;

#[cfg(test)]
thread_local! {
    static PRE_SPAWN_HOOK: RefCell<Option<ProcessHook>> = RefCell::new(None);
    static POST_SPAWN_HOOK: RefCell<Option<ProcessHook>> = RefCell::new(None);
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

#[cfg(not(test))]
fn run_test_process_pre_spawn_hook() {}

#[cfg(not(test))]
fn run_test_process_post_spawn_hook() {}

#[cfg(target_os = "macos")]
pub(crate) use darwin_child_custody::*;
#[cfg(target_os = "macos")]
pub(crate) use darwin_suspended_launch::*;
#[cfg(target_os = "macos")]
pub(crate) use loaded_executable_identity::*;
#[cfg(target_os = "macos")]
pub(crate) use object_bound_launch::*;
pub(crate) use process_execution::*;
pub(crate) use process_group_observation::*;
pub(crate) use process_input_write::*;
pub(crate) use process_output_drain::*;
pub(crate) use spawn_test_observation::*;

#[cfg(test)]
#[path = "process_object_binding_tests.rs"]
mod process_object_binding_tests;
#[cfg(test)]
#[path = "process_termination_tests.rs"]
mod process_termination_tests;
