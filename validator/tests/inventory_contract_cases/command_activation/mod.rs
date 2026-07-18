use crate::context::LiveContext;
use crate::inventory::{ActiveStatus, InventoryBuilder};
use crate::repository_fixture::{TestRepo, inventory_request, live_root, snapshot};
use serde::Deserialize;
use std::fs;

include!("command_activation/sources.rs");

include!("command_activation/absent_or_stale_witness_sources_fail_closed_without_echoing_bytes.rs");

include!("command_activation/projection_capture_rejects_restore_substitution_and_special_files.rs");
