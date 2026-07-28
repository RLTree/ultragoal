use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
#[cfg(unix)]
use std::ffi::{CStr, CString};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};

use crate::routine_work::{RepoPath, RoutineError, RoutineErrorId};

use super::super::execution_authority::{RoutineReadAncestor, RoutineReadSource};
use super::outcome::OutputFileRecord;

#[cfg(test)]
#[path = "confinement_transitions.rs"]
mod confinement_transitions;
#[path = "directory_read_failure.rs"]
mod directory_read_failure;
#[path = "executable_bound.rs"]
mod executable_bound;
#[path = "executable_identity.rs"]
mod executable_identity;
#[path = "framed_read_input.rs"]
mod framed_read_input;
#[path = "mediation_failure.rs"]
mod mediation_failure;
#[path = "output_file_limit.rs"]
mod output_file_limit;
#[path = "output_tree_capture.rs"]
mod output_tree_capture;
#[path = "ownership_rejection.rs"]
mod ownership_rejection;
#[path = "read_confinement.rs"]
mod read_confinement;
#[path = "read_source_opening.rs"]
mod read_source_opening;
#[path = "source_revalidation.rs"]
mod source_revalidation;

#[cfg(test)]
pub(crate) use confinement_transitions::*;
pub(crate) use directory_read_failure::*;
pub(crate) use mediation_failure::*;
pub(crate) use output_file_limit::*;
pub(crate) use output_tree_capture::*;
pub(crate) use ownership_rejection::*;
pub(crate) use read_source_opening::*;
pub(crate) use source_revalidation::*;
