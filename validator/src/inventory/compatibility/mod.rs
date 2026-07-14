#[path = "agent/specs.rs"]
mod agent_specs;
#[path = "agent/witness.rs"]
mod agent_witness;
mod reader_witness;
mod reader_witness_specs;
mod specs;
mod witness;

pub(super) use agent_witness::{
    AgentRouteApplication, agent_registry_route_is_compiled, apply_agent_route,
};
pub(super) use reader_witness::reader_proof_current;
pub(super) use specs::{
    RETAINED_KIND, RouteSpec, STRUCTURAL_WITNESS_KIND, by_legacy_name, by_route_id,
};
pub(super) use witness::{
    apply_route, inspect_wrapper, registry_route_is_compiled, retention_candidates,
};
