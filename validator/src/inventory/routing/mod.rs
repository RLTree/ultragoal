use super::compatibility::{
    agent_registry_route_is_compiled, reader_proof_current, registry_route_is_compiled,
};
use super::fs::{PhysicalEntryDescriptor, physical_entry, read_bounded};
use super::routing_state::RouteTransition;
use super::types::{ActiveStatus, AuthorityState, InventoryEntry, InventoryError};
use crate::context::ReadSession;
use serde::Deserialize;
use std::fs;
use std::path::Path;

include!("routes_path.rs");

#[path = "apply.rs"]
mod apply;
#[path = "../routing_pending/mod.rs"]
mod pending;

include!("route_match.rs");

include!("load.rs");
