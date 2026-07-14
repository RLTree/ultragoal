use crate::context::LiveContext;
use crate::inventory::{AuthorityCatalog, InventoryBuilder};
use crate::repository_fixture::{TestRepo, inventory_request, snapshot};
use sha2::{Digest, Sha256};
use std::fs;

include!("fixtures/catalog.rs");

include!("fixtures/hardlinked_inventory_content_is_rejected.rs");
