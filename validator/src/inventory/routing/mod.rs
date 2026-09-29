use super::routing_state::RouteTransition;
use super::types::{InventoryEntry, InventoryError};
use super::{MIGRATION_REGISTRY_PATH, ObservedMigrationRegistry};
use crate::context::ReadSession;
use serde::Deserialize;
use std::path::Path;

#[path = "apply.rs"]
mod apply;
#[path = "../routing_pending/mod.rs"]
mod pending;

include!("route_match.rs");

include!("load.rs");
