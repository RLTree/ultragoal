#![cfg(target_vendor = "apple")]

use super::repository_fit::{
    CanonicalPath, FitErrorId, FitMode, FitReader, LocalRepository, Ownership, RepositoryClass,
    inspect, plan, verify,
};
use super::scenario::{desired, file};
use std::ffi::CString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "local_read_cases/scenario_fixture.rs"]
mod scenario_fixture;
#[path = "local_read_cases/snapshot_contract.rs"]
mod snapshot_contract;

pub(crate) use snapshot_contract::*;
