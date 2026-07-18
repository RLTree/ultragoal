use super::super::{AgentAuthorityLayer, AgentDiscoveryErrorId, AgentDiscoverySession};
use super::authority_fixtures::{FixtureReader, TempRepo, descriptor, digest};
use serde_json::json;

include!("live_legacy_agents.rs");

include!("wrong_project_candidate_session_nonce_or_fresh_session_identity_is_rejected.rs");
