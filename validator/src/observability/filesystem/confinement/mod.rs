use std::fs::{File, Metadata};

#[cfg(unix)]
use std::ffi::{CStr, CString};
#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use std::path::{Component, Path};

use super::FileIdentity;
#[cfg(unix)]
use super::anchors::{
    absolute_parent, directory_identity, identity_stat, validate_directory_link, validate_stat,
};

#[path = "created_leaf_cleanup.rs"]
mod created_leaf_cleanup;
#[path = "parent_confinement.rs"]
mod parent_confinement;

pub(crate) use created_leaf_cleanup::*;
pub(crate) use parent_confinement::*;
