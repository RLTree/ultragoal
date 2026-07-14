use super::fixtures::{command, live_loop_timing_receipt_arg, live_loop_timing_receipt_path};
use crate::cli::live_loop::changed_inputs::ChangedInputs;
use crate::cli::live_loop::surfaces::{LoopValidationSurface, input_spec_for, surface_by_id};
use serde_json::json;
use std::path::Path;

#[path = "receipt_path.rs"]
mod receipt_path;
#[path = "replay_cache.rs"]
mod replay_cache;

pub(crate) use receipt_path::*;
pub(crate) use replay_cache::*;
