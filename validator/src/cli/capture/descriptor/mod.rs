use super::identity_codec::digest_bytes;
use std::fs::{File, Metadata};

#[cfg(unix)]
use std::ffi::CString;
#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::{FileExt, MetadataExt};

#[path = "bounded_descriptor_read.rs"]
mod bounded_descriptor_read;
#[path = "descriptor_capture.rs"]
mod descriptor_capture;

pub(crate) use descriptor_capture::*;
