use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use ultragoal::routine_work::{
    CheckClass, CheckNode, ImpactGraph, PathMatcher, PathRoute, RepoPath, RunnerSpec,
};

#[path = "production_scenario_cases/argument_fixture.rs"]
mod argument_fixture;
#[path = "production_scenario_cases/contender_process/mod.rs"]
mod contender_process;
#[path = "production_scenario_cases/execution_fixture.rs"]
mod execution_fixture;
#[path = "production_scenario_cases/scenario_fixture.rs"]
mod scenario_fixture;

pub(crate) use argument_fixture::*;
pub(crate) use contender_process::*;
pub(crate) use execution_fixture::*;
pub(crate) use scenario_fixture::*;
