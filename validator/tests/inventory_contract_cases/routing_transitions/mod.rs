use crate::context::LiveContext;
use crate::inventory::{ActiveStatus, AuthorityState, InventoryBuilder};
use crate::repository_fixture::{TestRepo, inventory_request};
use std::fs;

include!("catalog.rs");

include!("typed_route_states_cannot_launder_generic_or_unproven_retirement.rs");
