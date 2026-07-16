use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{CStr, CString};
use std::fs::{self, File};
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use super::custody::{
    OutputComponentJournal, OutputDirectoryIdentity, OutputProvisionJournal, OutputStageAmbiguity,
};
use super::production_mediation::error;
use crate::routine_work::{RepoPath, RoutineError};

#[path = "apply.rs"]
mod apply;
#[path = "creation.rs"]
mod creation;
#[path = "directory_entries.rs"]
mod directory_entries;
#[path = "observation.rs"]
mod observation;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ApplyOutcome {
    Applied,
    UnrecordedStage(OutputStageAmbiguity),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum OutputTransition {
    Staged {
        relative_path: String,
        identity: OutputDirectoryIdentity,
    },
    Published {
        relative_path: String,
        identity: OutputDirectoryIdentity,
    },
}

impl ApplyOutcome {
    pub(super) fn into_ambiguity(self) -> Option<OutputStageAmbiguity> {
        match self {
            Self::Applied => None,
            Self::UnrecordedStage(ambiguity) => Some(ambiguity),
        }
    }
}

pub(super) fn observe(
    root: &Path,
    scopes: &[RepoPath],
) -> Result<OutputProvisionJournal, RoutineError> {
    observation::observe(root, scopes)
}

pub(super) fn begin(
    journal: &OutputProvisionJournal,
    root: &Path,
) -> Result<apply::OutputProvisioning, RoutineError> {
    apply::begin(journal, root)
}

pub(super) use apply::OutputStep;

fn identity(metadata: &fs::Metadata) -> OutputDirectoryIdentity {
    OutputDirectoryIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        owner: metadata.uid(),
        mode: metadata.mode(),
    }
}

fn validate_name(name: &str) -> Result<CString, RoutineError> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.as_bytes().contains(&0)
    {
        return Err(error("routine-production-output-name-invalid"));
    }
    CString::new(name).map_err(|_| error("routine-production-output-name-invalid"))
}
