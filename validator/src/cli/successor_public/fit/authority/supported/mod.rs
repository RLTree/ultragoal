use super::{production_outcome, HostFailure};
use crate::cli::successor::runtime::RuntimeOutcome;
use crate::context::LiveContext;
use crate::repository_fit::{
    digest, execute_prepared_apply, prepare_recovery_intent, recover_prepared_apply, valid_digest,
    FitAdapterError, PreparedFitApply, RepositoryFitApplyNonce, RepositoryFitAuthorityStore,
    RepositoryFitProductionOutcome, RepositoryFitTrustedClock,
};
use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

#[path = "anchored_directory.rs"]
mod anchored_directory;
#[path = "openat.rs"]
mod openat;
#[path = "pending_publication.rs"]
mod pending_publication;
#[path = "process_lock.rs"]
mod process_lock;
#[path = "state_components.rs"]
mod state_components;
#[path = "state_open.rs"]
mod state_open;
#[cfg(test)]
#[path = "target_confinement.rs"]
mod target_confinement;

pub(crate) use anchored_directory::*;
pub(crate) use openat::*;
pub(crate) use pending_publication::*;
pub(crate) use process_lock::*;
pub(crate) use state_components::*;
