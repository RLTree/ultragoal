use crate::context::LiveContext;
use crate::inventory::InventoryBuilder;
use crate::repository_fixture::{TestRepo, inventory_request};

include!("manifest.rs");

include!("parsed_but_skipped_hook_configuration_remains_an_explicit_warning.rs");
