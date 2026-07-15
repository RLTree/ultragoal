pub mod context {
    pub use ultragoal::context::*;
}

#[path = "../src/cli/capture/mod.rs"]
pub mod capture;

#[path = "../src/routine_work/mod.rs"]
pub mod routine_work;

#[path = "routine_work_contract/authority_ledger_controls.rs"]
mod authority_ledger_controls;
#[path = "routine_work_contract/broker_binding_controls.rs"]
mod broker_binding_controls;
#[path = "routine_work_contract/configured_path_alias.rs"]
mod configured_path_alias;
#[path = "routine_work_contract/contract.rs"]
mod contract;
#[path = "routine_work_contract/filesystem_controls.rs"]
mod filesystem_controls;
#[path = "routine_work_contract/issuer_api_compilation.rs"]
mod issuer_api_compilation;
#[path = "routine_work_contract/issuer_api_visibility.rs"]
mod issuer_api_visibility;
#[path = "routine_work_contract/issuer_capture_races.rs"]
mod issuer_capture_races;
#[path = "routine_work_contract/issuer_cleanup_races.rs"]
mod issuer_cleanup_races;
#[path = "routine_work_contract/issuer_hidden_surface.rs"]
mod issuer_hidden_surface;
#[path = "routine_work_contract/issuer_reconciliation_races.rs"]
mod issuer_reconciliation_races;
#[path = "routine_work_contract/issuer_rollback_races.rs"]
mod issuer_rollback_races;
#[path = "routine_work_contract/issuer_scratch_resilience.rs"]
mod issuer_scratch_resilience;
#[path = "routine_work_contract/local_capture.rs"]
mod local_capture;
#[path = "routine_work_contract/owned_compile_claim.rs"]
mod owned_compile_claim;
#[path = "routine_work_contract/owned_compile_directory.rs"]
mod owned_compile_directory;
#[path = "routine_work_contract/owned_compile_retention.rs"]
mod owned_compile_retention;
#[path = "routine_work_contract/owned_compile_scratch.rs"]
mod owned_compile_scratch;
#[path = "routine_work_contract/planning.rs"]
mod planning;
#[path = "routine_work_contract/provenance.rs"]
mod provenance;
#[path = "routine_work_contract/report.rs"]
mod report;
#[path = "routine_work_contract/retired_adapter_routes.rs"]
mod retired_adapter_routes;
#[path = "routine_work_contract/retired_authority_routes.rs"]
mod retired_authority_routes;
#[path = "routine_work_contract/retired_behavior_routes.rs"]
mod retired_behavior_routes;
#[path = "routine_work_contract/retired_process_lifecycle_routes.rs"]
mod retired_process_lifecycle_routes;
#[path = "routine_work_contract/retired_reuse_output_routes.rs"]
mod retired_reuse_output_routes;
#[path = "routine_work_contract/reuse/mod.rs"]
mod reuse;
#[path = "routine_work_contract/routine_fixture_repository.rs"]
mod routine_fixture_repository;
#[path = "routine_work_contract/routine_fixture_workspace.rs"]
mod routine_fixture_workspace;
#[path = "routine_work_contract/routine_plan_fixture.rs"]
mod routine_plan_fixture;
#[path = "routine_work_contract/scenario.rs"]
mod scenario;
#[path = "routine_work_contract/security.rs"]
mod security;
