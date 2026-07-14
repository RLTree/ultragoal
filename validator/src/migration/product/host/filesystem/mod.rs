use super::{HostError, MAX_HOST_FILE_BYTES};
use std::ffi::CString;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

include!("file_identity.rs");

include!("anchored/absolute_open.rs");

include!("anchored/atomic_write.rs");

include!("process_lock_acquire.rs");
