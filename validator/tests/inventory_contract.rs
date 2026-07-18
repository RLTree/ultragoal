mod context {
    pub use ultragoal::context::*;
}

mod inventory {
    pub use ultragoal::inventory::*;
}

#[path = "inventory_contract_cases/agent_manifest.rs"]
mod agent_manifest;
#[path = "inventory_contract_cases/agent_routes/mod.rs"]
mod agent_routes;
#[path = "inventory_contract_cases/command_activation/mod.rs"]
mod command_activation;
#[path = "inventory_contract_cases/compatibility_routes/mod.rs"]
mod compatibility_routes;
#[path = "inventory_contract_cases/context_scopes/mod.rs"]
mod context_scopes;
#[path = "inventory_contract_cases/contract_integrity.rs"]
mod contract_integrity;
#[path = "inventory_contract_cases/fixtures/mod.rs"]
mod fixtures;
#[path = "inventory_contract_cases/generated_disposition/mod.rs"]
mod generated_disposition;
#[path = "inventory_contract_cases/generated_regeneration.rs"]
mod generated_regeneration;
#[path = "inventory_contract_cases/legacy_limits.rs"]
mod legacy_limits;
#[path = "inventory_contract_cases/legacy_scope.rs"]
mod legacy_scope;
#[path = "inventory_contract_cases/plugin_hooks.rs"]
mod plugin_hooks;
#[path = "inventory_contract_cases/plugin_manifest/mod.rs"]
mod plugin_manifest;
#[path = "inventory_contract_cases/plugin_manifest_semantics.rs"]
mod plugin_manifest_semantics;
#[path = "inventory_contract_cases/repository_fixture.rs"]
mod repository_fixture;
#[path = "inventory_contract_cases/routing_transitions/mod.rs"]
mod routing_transitions;
#[path = "inventory_contract_cases/schema_references.rs"]
mod schema_references;

use context::LiveContext;
use inventory::{ActiveStatus, AuthorityState, InventoryBuilder};

include!(
    "inventory_contract/current_live_catalog_reports_contract_definitions_and_missing_topology.rs"
);
