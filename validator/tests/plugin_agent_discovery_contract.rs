#[path = "../src/plugin_product/agent_discovery/mod.rs"]
pub(crate) mod agent_discovery;
#[path = "../src/agent_roles.rs"]
mod agent_roles;
#[path = "../src/plugin_manifest/mod.rs"]
mod plugin_manifest;

mod plugin_product {
    pub(crate) use crate::agent_discovery;
}

#[path = "plugin_agent_discovery_contract/authority_fixtures/mod.rs"]
mod authority_fixtures;
#[path = "plugin_agent_discovery_contract/false_pass.rs"]
mod false_pass;
#[path = "plugin_agent_discovery_contract/manifest_shape.rs"]
mod manifest_shape;
#[path = "plugin_agent_discovery_contract/negative/mod.rs"]
mod negative;
#[path = "plugin_agent_discovery_contract/positive.rs"]
mod positive;
#[path = "plugin_agent_discovery_contract/race.rs"]
mod race;
#[path = "plugin_agent_discovery_contract/security/mod.rs"]
mod security;
#[path = "plugin_agent_discovery_contract/supported_host/mod.rs"]
mod supported_host;
