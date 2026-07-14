mod evidence;
mod exact_set;
mod manifest;
mod proposal_context;

use super::fs::{PhysicalEntryDescriptor, physical_regular_entry, read_bounded};
use super::types::{
    ActiveStatus, AuthorityState, InventoryEntry, InventoryError, InventoryFinding,
};
use crate::context::ReadSession;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

include!("registry_relative.rs");

include!("load.rs");
