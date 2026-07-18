use crate::context::LiveContext;
use crate::inventory::{ActiveStatus, AuthorityState, InventoryBuilder};
use crate::repository_fixture::{TestRepo, inventory_request, live_root};
use serde_json::{Value, json};
use std::fs;

include!("route_id.rs");

include!("registry_text_cannot_launder_a_non_wrapper_or_missing_target.rs");
