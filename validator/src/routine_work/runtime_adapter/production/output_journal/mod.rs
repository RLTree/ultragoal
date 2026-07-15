use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{CStr, CString};
use std::fs::{self, File};
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use super::ledger::{
    FileAuthorityLedger, OutputComponentJournal, OutputDirectoryIdentity, OutputProvisionJournal,
    ReservationToken,
};
use super::production_mediation::error;
use crate::routine_work::{RepoPath, RoutineError};

#[path = "apply.rs"]
mod apply;
#[path = "creation.rs"]
mod creation;
#[cfg(test)]
#[path = "custody_tests.rs"]
mod custody_tests;
#[path = "directory_entries.rs"]
mod directory_entries;
#[path = "observation.rs"]
mod observation;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub(super) fn observe(
    root: &Path,
    scopes: &[RepoPath],
) -> Result<OutputProvisionJournal, RoutineError> {
    observation::observe(root, scopes)
}

pub(super) fn apply(
    ledger: &FileAuthorityLedger,
    token: &ReservationToken,
    root: &Path,
) -> Result<(), RoutineError> {
    apply::apply(ledger, token, root)
}

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
