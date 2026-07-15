use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
#[cfg(test)]
use std::sync::{Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::os::unix::process::{CommandExt, ExitStatusExt};

use crate::routine_work::{RoutineError, RoutineErrorId};

use super::filesystem::{OutputConfinement, PinnedExecutable, ReadConfinement, RootAnchor};
use super::outcome::RoutineCancellation;

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
static PRE_SPAWN_HOOK: OnceLock<Mutex<Option<ProcessHook>>> = OnceLock::new();
#[cfg(test)]
static POST_SPAWN_HOOK: OnceLock<Mutex<Option<ProcessHook>>> = OnceLock::new();

#[cfg(test)]
pub(crate) fn set_test_process_pre_spawn_hook(hook: impl FnOnce() + Send + 'static) {
    *PRE_SPAWN_HOOK
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Box::new(hook));
}

#[cfg(test)]
pub(crate) fn set_test_process_post_spawn_hook(hook: impl FnOnce() + Send + 'static) {
    *POST_SPAWN_HOOK
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Box::new(hook));
}

#[cfg(test)]
fn run_test_process_pre_spawn_hook() {
    if let Some(hook) = PRE_SPAWN_HOOK
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take()
    {
        hook();
    }
}

#[cfg(test)]
fn run_test_process_post_spawn_hook() {
    if let Some(hook) = POST_SPAWN_HOOK
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take()
    {
        hook();
    }
}

#[cfg(not(test))]
fn run_test_process_pre_spawn_hook() {}

#[cfg(not(test))]
fn run_test_process_post_spawn_hook() {}

pub(crate) use process_execution::*;
pub(crate) use process_group_observation::*;
pub(crate) use process_input_write::*;
pub(crate) use process_output_drain::*;
pub(crate) use spawn_test_observation::*;

#[cfg(test)]
#[path = "process_object_binding_tests.rs"]
mod process_object_binding_tests;
