mod context {
    pub use ultragoal::context::*;
}

#[path = "cli_contract/context_boundaries.rs"]
mod context_boundaries;
#[path = "cli_contract/context_identity.rs"]
mod context_identity;
#[path = "cli_contract/context_scenario.rs"]
mod context_scenario;
pub(crate) use context_scenario::serial;

#[path = "cli_contract/state/mod.rs"]
mod state_contract;
#[path = "cli_contract/capture/mod.rs"]
mod successor_capture;
#[path = "cli_contract/successor/mod.rs"]
mod successor_cli_contract;
