use crate::context::LiveContext;
use crate::inventory::InventoryBuilder;
use crate::repository_fixture::{TestRepo, inventory_request, live_root};
use serde_json::{Value, json};
use std::fs;

include!("registry_fixture/reader_proof.rs");

include!("registry_fixture/write_registry.rs");
