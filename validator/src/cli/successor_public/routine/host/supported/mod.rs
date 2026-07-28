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
#[path = "state_components.rs"]
mod state_components;
#[path = "validate_name.rs"]
mod validate_name;

pub(crate) use anchored_directory::write_lock_marker;
pub(crate) use continuity::{ContinuationCheckpoint, ContinuationResolution};
pub(crate) use state_components::*;
pub(crate) use validate_name::*;
