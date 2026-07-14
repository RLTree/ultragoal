use crate::context::LiveContext;
use crate::inventory::{
    ActiveStatus, AuthorityCatalog, AuthorityState, InventoryBuilder, InventoryError,
};
use crate::repository_fixture::{TestRepo, inventory_request, live_root};
use serde_json::{Value, json};
use std::fs;

include!("context_scopes/candidate_root.rs");

include!("context_scopes/tamper_extra_missing_and_manifest_changes_fail_closed.rs");

#[path = "context_scopes/v2.rs"]
mod v2;
