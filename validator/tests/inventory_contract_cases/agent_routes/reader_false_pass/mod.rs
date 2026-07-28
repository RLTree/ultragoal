use super::prepare;
use super::registry_fixture::{CASES, READER_PROOF, catalog, catalog_result, entry};
use crate::inventory::{ActiveStatus, AuthorityState};
use crate::repository_fixture::TestRepo;
use serde_json::{Value, json};
use std::fs;

include!("assert_catalog_blocks_route.rs");

include!("constructed_agent_paths_cannot_bypass_the_reader_guard.rs");

include!("symlink_hardlink_fifo_and_socket_agent_entries_block_without_opening_specials.rs");
