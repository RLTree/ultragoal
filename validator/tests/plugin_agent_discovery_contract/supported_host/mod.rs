use crate::agent_discovery::{
    AgentAuthorityLayer, AgentDiscoveryErrorId, AgentDiscoverySession,
    SupportedAgentAuthorityFindingKind, SupportedHostAgentAuthorityReader,
};
use crate::authority_fixtures::{
    SupportedHostFixture, TempRepo, canonical_names, descriptor, tree_snapshot,
};
use std::collections::BTreeSet;
use std::fs;

include!("exact_reader.rs");

include!("missing_extra_and_write_capable_authority_are_reported_exactly.rs");

include!("socket_and_unreadable_host_entries_are_refused_without_blocking.rs");
