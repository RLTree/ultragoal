use super::spec::stable_digest;
use super::{FixtureScheduleError, FixtureSpec, ResourceKind};
use std::collections::BTreeMap;
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::ffi::{CStr, CString, OsStr, OsString};
#[cfg(unix)]
use std::io;
#[cfg(unix)]
use std::mem::MaybeUninit;
#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
#[cfg(unix)]
use std::os::unix::ffi::{OsStrExt, OsStringExt};

include!("before_capture_hook.rs");

include!("pinned_lease_root_pin.rs");

include!("capture_and_remove.rs");

include!("rename_noreplace.rs");
