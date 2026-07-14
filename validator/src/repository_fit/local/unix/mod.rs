use super::super::{CanonicalPath, FitError, FitErrorId, error};
use super::sys::{
    EntryMatch, EnumerationBudget, PathStat, duplicate, exact_entry, open_at, stat_at,
};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
#[path = "../apple_path.rs"]
mod platform_path;
use platform_path::descriptor_path;

#[cfg(all(test, target_vendor = "apple"))]
#[path = "../unix_tests/mod.rs"]
mod tests;

#[path = "bounded_file_read.rs"]
mod bounded_file_read;
#[path = "workspace_authority.rs"]
mod workspace_authority;
#[path = "workspace_opening.rs"]
mod workspace_opening;

pub(crate) use bounded_file_read::*;
pub(crate) use workspace_authority::*;
