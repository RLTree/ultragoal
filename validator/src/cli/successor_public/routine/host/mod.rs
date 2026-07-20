use std::path::Path;

#[cfg(target_vendor = "apple")]
#[path = "supported/mod.rs"]
mod supported;

pub(crate) use supported::ContinuationCheckpoint;

#[path = "host_failure.rs"]
mod host_failure;

pub(crate) use host_failure::*;
