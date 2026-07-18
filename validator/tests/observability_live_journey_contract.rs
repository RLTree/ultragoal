#![cfg(unix)]

use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ffi::CString;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use ultragoal::observability::{EventQuery, EventStore, SemanticEvent, SemanticEventInput};

#[path = "cli_contract/state/authority_inputs.rs"]
mod authority_inputs;
#[path = "observability_live_journey_cases/causal_correlation_journey.rs"]
mod causal_correlation_journey;
#[path = "observability_authority_fixture.rs"]
mod observability_authority_fixture;
#[path = "observability_live_journey_cases/process_output_fixture.rs"]
mod process_output_fixture;
#[path = "observability_live_journey_cases/scenario_fixture.rs"]
mod scenario_fixture;
#[path = "observability_live_journey_cases/selected_finding_fixture.rs"]
mod selected_finding_fixture;
#[path = "observability_live_journey_cases/substitution_refusals.rs"]
mod substitution_refusals;

pub(crate) use process_output_fixture::*;
pub(crate) use scenario_fixture::*;
pub(crate) use selected_finding_fixture::*;
