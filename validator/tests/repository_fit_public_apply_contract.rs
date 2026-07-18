#![cfg(target_vendor = "apple")]

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

#[path = "repository_fit_public_apply_cases/execution_fixture.rs"]
mod execution_fixture;
#[path = "repository_fit_public_apply_cases/filesystem_snapshot.rs"]
mod filesystem_snapshot;
#[path = "repository_fit_public_apply_cases/public_apply_refusals.rs"]
mod public_apply_refusals;

pub(crate) use execution_fixture::*;
pub(crate) use filesystem_snapshot::*;
pub(crate) use public_apply_refusals::*;
