use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use ultragoal::observability::{EventStore, SemanticEvent, SemanticEventInput};

#[path = "fixture_components/process_output_fixture.rs"]
mod process_output_fixture;
#[path = "fixture_components/scenario_fixture.rs"]
mod scenario_fixture;

pub(crate) use process_output_fixture::*;
pub(crate) use scenario_fixture::*;
