//! Std-only build-time classification and staging for repository-fit sources.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};

#[path = "repository_fit_templates/manifest_size_limit.rs"]
mod manifest_size_limit;
#[path = "repository_fit_templates/source_open_policy.rs"]
mod source_open_policy;
#[path = "repository_fit_templates/source_staging.rs"]
mod source_staging;
#[path = "repository_fit_templates/source_tree_inspection.rs"]
mod source_tree_inspection;
#[path = "repository_fit_templates/staged_publication.rs"]
mod staged_publication;

pub(crate) use manifest_size_limit::*;
#[cfg(not(target_os = "macos"))]
pub(crate) use source_open_policy::*;
pub(crate) use source_staging::*;
pub(crate) use source_tree_inspection::*;
pub(crate) use staged_publication::*;
