use crate::context::LiveContext;
use crate::inventory::{ActiveStatus, InventoryBuilder};
use crate::repository_fixture::{TestRepo, inventory_request, live_root, snapshot};
use serde::Deserialize;
use std::fs;

include!("worktree_authority/fixture.rs");
include!("worktree_authority/issuance.rs");
include!("worktree_authority/envelopes.rs");
include!("sources.rs");
include!("unsupported_product_apis_remain_inactive.rs");

include!("absent_or_stale_witness_sources_fail_closed_without_echoing_bytes.rs");

include!("projection_capture_rejects_restore_substitution_and_special_files.rs");
