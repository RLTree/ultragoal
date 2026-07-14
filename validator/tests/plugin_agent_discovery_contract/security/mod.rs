use crate::agent_discovery::{
    AgentAuthorityLayer, AgentDiscoveryErrorId, AgentDiscoverySession, ReaddirTestFault,
    SourceAgentCatalog, reset_test_io_counts, set_test_readdir_fault, test_io_counts,
    test_readdir_fault_triggered,
};
use crate::authority_fixtures::{
    CANDIDATE, FixtureReader, SESSION, TempRepo, descriptor, tree_snapshot,
};
use std::fs;

include!("source_symlink_and_hardlink_descriptors_are_rejected.rs");

include!("unicode_and_oversized_unexpected_names_fail_before_source_body_reads.rs");

include!("errors_do_not_echo_host_or_descriptor_input.rs");
