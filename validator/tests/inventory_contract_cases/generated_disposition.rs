use crate::context::LiveContext;
use crate::inventory::{ActiveStatus, AuthorityState, InventoryBuilder};
use crate::repository_fixture::{TestRepo, inventory_request};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;

include!("generated_disposition/contract_id.rs");

include!("generated_disposition/generated_authority_v2_keeps_registry_and_collection_bounds.rs");
