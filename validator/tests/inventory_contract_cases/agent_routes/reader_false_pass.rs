use super::prepare;
use super::registry_fixture::{CASES, READER_PROOF, catalog, catalog_result, entry};
use crate::inventory::{ActiveStatus, AuthorityState};
use crate::repository_fixture::TestRepo;
use serde_json::{Value, json};
use std::fs;

include!("reader_false_pass/assert_catalog_blocks_route.rs");

include!("reader_false_pass/constructed_agent_paths_cannot_bypass_the_reader_guard.rs");

include!(
    "reader_false_pass/symlink_hardlink_fifo_and_socket_agent_entries_block_without_opening_specials.rs"
);
