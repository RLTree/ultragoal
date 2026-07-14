#![allow(dead_code, unused_imports)]

mod context {
    pub use ultragoal::context::*;
}

#[path = "../src/cli/capture/mod.rs"]
mod capture;

#[path = "../src/routine_work/mod.rs"]
mod routine_work;

#[path = "routine_work_contract/authority_ledger_controls.rs"]
mod authority_ledger_controls;
#[path = "routine_work_contract/contract.rs"]
mod contract;
#[path = "routine_work_contract/current_path_controls.rs"]
mod current_path_controls;
#[path = "routine_work_contract/current_path_fixture.rs"]
mod current_path_fixture;
#[path = "routine_work_contract/filesystem_controls.rs"]
mod filesystem_controls;
#[path = "routine_work_contract/invariant_control_map.rs"]
mod invariant_control_map;
#[path = "routine_work_contract/invariant_control_map_adapter.rs"]
mod invariant_control_map_adapter;
#[path = "routine_work_contract/invariant_control_map_authority.rs"]
mod invariant_control_map_authority;
#[path = "routine_work_contract/invariant_control_map_mediator_a.rs"]
mod invariant_control_map_mediator_a;
#[path = "routine_work_contract/invariant_control_map_mediator_b.rs"]
mod invariant_control_map_mediator_b;
#[path = "routine_work_contract/issuer_visibility.rs"]
mod issuer_visibility;
#[path = "routine_work_contract/local_capture.rs"]
mod local_capture;
#[path = "routine_work_contract/planning.rs"]
mod planning;
#[path = "routine_work_contract/provenance.rs"]
mod provenance;
#[path = "routine_work_contract/provenance_compile.rs"]
mod provenance_compile;
#[path = "routine_work_contract/report.rs"]
mod report;
#[path = "routine_work_contract/reuse/mod.rs"]
mod reuse;
#[path = "routine_work_contract/scenario.rs"]
mod scenario;
#[path = "routine_work_contract/security.rs"]
mod security;
