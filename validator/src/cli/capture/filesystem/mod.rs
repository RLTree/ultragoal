use super::descriptor::{DirectoryIdentity, Snapshot, open_dir_at, open_file_at, read_descriptor};
use super::path_policy::{validate_public_path, validate_relative};
use crate::context::LiveContext;
use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[cfg(test)]
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

#[path = "pinned_file_access.rs"]
mod pinned_file_access;
#[path = "root_anchor.rs"]
mod root_anchor;

pub(crate) use pinned_file_access::*;
pub(crate) use root_anchor::*;
