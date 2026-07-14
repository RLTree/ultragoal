use super::{FixtureCaptureAdapter, root, spec};
use crate::fixture_scheduler::*;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

include!("timeout_spec.rs");

include!("actual_adapter_termination.rs");

include!("already_exited_process_group_is_absent_after_adapter_return.rs");
