mod context {
    pub use ultragoal::context::*;
}

#[path = "routine_work_contract/scenario.rs"]
mod base_support;
#[path = "../src/cli/capture/mod.rs"]
mod capture;
#[path = "routine_recovery_reuse_journey_contract/catalog.rs"]
mod catalog;
#[path = "../src/digest.rs"]
mod digest;
#[path = "../src/fixture_scheduler/mod.rs"]
mod fixture_scheduler;
#[path = "routine_recovery_reuse_journey_contract/journey_scenario.rs"]
mod journey_scenario;
#[path = "routine_recovery_reuse_journey_contract/journeys.rs"]
mod journeys;
#[path = "../src/orchestration/mod.rs"]
mod orchestration;
#[path = "routine_recovery_reuse_journey_contract/routine/fixture/inventory.rs"]
mod routine_fixture_inventory;
#[path = "routine_recovery_reuse_journey_contract/routine/fixture/invocation.rs"]
mod routine_fixture_invocation;
#[path = "routine_recovery_reuse_journey_contract/routine/fixture/lifecycle.rs"]
mod routine_fixture_lifecycle;
#[path = "routine_work_contract/routine/fixture_repository.rs"]
mod routine_fixture_repository;
#[path = "routine_work_contract/routine/fixture_workspace.rs"]
mod routine_fixture_workspace;
#[path = "../src/routine_work/mod.rs"]
mod routine_work;
