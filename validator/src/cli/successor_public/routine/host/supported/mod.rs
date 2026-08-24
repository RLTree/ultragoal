use super::HostFailure;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

mod anchored_directory;
mod continuity;
#[path = "directory_entries.rs"]
mod directory_entries;
#[path = "host_state.rs"]
mod host_state;
#[path = "migration_admission.rs"]
mod migration_admission;
#[path = "parent_state_lock.rs"]
mod parent_state_lock;
#[path = "quarantine_apply.rs"]
mod quarantine_apply;
#[path = "quarantine_transition.rs"]
mod quarantine_transition;
#[path = "reserved_recovery_admission.rs"]
mod reserved_recovery_admission;
#[path = "state_components.rs"]
mod state_components;
#[path = "validate_name.rs"]
mod validate_name;

#[cfg(test)]
pub(crate) use anchored_directory::{fail_after_next_rename, fail_before_next_rename};
pub(crate) use anchored_directory::write_lock_marker;
pub(crate) use anchored_directory::{ExclusivePublishFailure, ExclusivePublishSite};
pub(crate) use continuity::{ContinuationCheckpoint, ContinuationResolution};
pub(crate) use migration_admission::assess_migration_admission;
pub(crate) use parent_state_lock::ParentStateLock;
pub(crate) use quarantine_apply::apply_quarantine_plan;
pub(crate) use state_components::*;
pub(crate) use validate_name::*;
