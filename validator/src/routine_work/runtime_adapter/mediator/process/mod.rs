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

#[cfg(target_os = "macos")]
#[path = "child_authority_channel.rs"]
mod child_authority_channel;
#[cfg(test)]
#[path = "broker_gate_probe.rs"]
mod broker_gate_probe;
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

#[cfg(target_os = "macos")]
pub(crate) use child_authority_channel::*;
#[cfg(test)]
pub(crate) use broker_gate_probe::*;
pub(crate) use process_execution::*;
pub(crate) use process_group_observation::*;
pub(crate) use process_input_write::*;
pub(crate) use process_output_drain::*;
pub(crate) use spawn_test_observation::*;
