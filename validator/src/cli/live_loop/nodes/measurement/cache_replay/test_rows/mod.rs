use super::super::{CacheReplay, NODE_TIMING_REL, VALIDATION_CACHE_REL, verified_local_hit};
use crate::cli::live_loop::nodes::measurement::ObservationMode;
use crate::cli::live_loop::{LiveLoopAction, LiveLoopCommand, surfaces::surface_by_id};
use crate::self_tests::boundaries::workspace_fixtures::temp_root;
use serde_json::json;
use std::path::{Path, PathBuf};

#[path = "../test_process_receipt.rs"]
mod test_process_receipt;

#[path = "receipt_binding.rs"]
mod receipt_binding;
#[path = "replay_fixture.rs"]
mod replay_fixture;

pub(crate) use receipt_binding::*;
pub(crate) use replay_fixture::*;
