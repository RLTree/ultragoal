use crate::context::LiveContext;
use crate::inventory::InventoryBuilder;
use crate::repository_fixture::{TestRepo, inventory_request};

include!("plugin_manifest/manifest.rs");

include!("plugin_manifest/parsed_but_skipped_hook_configuration_remains_an_explicit_warning.rs");
