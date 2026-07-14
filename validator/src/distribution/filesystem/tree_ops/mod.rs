#[cfg(unix)]
use super::descriptor::{Directory, DirectoryIdentity, EntryKind, create_file, rename_noreplace};
#[cfg(unix)]
use super::remove::remove_tree;
use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::package::TreeObject;
use crate::distribution::reader::validate_relative_path;

include!("tree_snapshot.rs");

include!("transition_existing.rs");
