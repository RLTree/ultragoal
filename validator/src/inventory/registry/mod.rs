mod command_activation;
mod data;
mod integrity;
mod semantic;
mod sources;
mod topology;

use super::digest::json_digest;
use super::fs::{contract_source_entry, read_bounded};
use super::types::{
    ActiveStatus, AuthorityState, InventoryEntry, InventoryError, InventoryFinding,
};
use crate::context::ReadSession;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

include!("json.rs");

include!("load.rs");
pub(crate) use command_activation::{guard as guard_activation, revalidate_sources};
pub(crate) use data::{CONTRACT_DIR, MAX_CONTRACT_JSON_BYTES, RegistryData, safe_identifier};
