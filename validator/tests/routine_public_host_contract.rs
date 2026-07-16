#![cfg(target_vendor = "apple")]

use serde::{Deserialize, Serialize};
use std::path::Path;

#[path = "../src/cli/successor_public/routine/host/host_failure.rs"]
mod host_failure;
#[path = "../src/cli/successor_public/routine/host/supported/mod.rs"]
mod supported;

pub(crate) use host_failure::*;
