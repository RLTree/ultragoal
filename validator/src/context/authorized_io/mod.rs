use super::bound_context::EffectClass;
use super::error::{ContextError, io_error};
use super::path::AuthorizedPath;
use std::fs::{self, File};
use std::path::{Component, Path};

#[cfg(unix)]
use std::ffi::CString;
#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(any(target_os = "macos", target_os = "linux"))]
unsafe extern "C" {
    fn openat(directory: i32, path: *const std::ffi::c_char, flags: i32, ...) -> i32;
}

#[path = "open_directory_at.rs"]
mod open_directory_at;
#[path = "replacement_race_hook.rs"]
mod replacement_race_hook;

pub(crate) use open_directory_at::*;
pub(crate) use replacement_race_hook::*;
