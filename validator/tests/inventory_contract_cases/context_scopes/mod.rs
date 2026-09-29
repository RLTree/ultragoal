use crate::context::LiveContext;
use crate::inventory::{
    ActiveStatus, AuthorityCatalog, AuthorityState, InventoryBuilder, InventoryError,
};
use crate::repository_fixture::{TestRepo, inventory_request, live_root};
use serde_json::{Value, json};
use std::fs;

include!("candidate_root.rs");

include!("tamper_extra_missing_and_manifest_changes_fail_closed.rs");

#[path = "v2/mod.rs"]
mod v2;
