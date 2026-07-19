use crate::context::LiveContext;
use crate::inventory::{ActiveStatus, InventoryBuilder};
use crate::repository_fixture::{TestRepo, inventory_request, live_root, snapshot};
use serde::Deserialize;
use std::fs;

include!("lease_fixture.rs");
include!("lease_issuance.rs");
include!("lease_envelopes.rs");
include!("sources.rs");
include!("unsupported_product_apis_remain_inactive.rs");

include!("absent_or_stale_witness_sources_fail_closed_without_echoing_bytes.rs");

include!("projection_capture_rejects_restore_substitution_and_special_files.rs");
