#![allow(dead_code, unused_imports)]

mod context {
    pub use ultragoal::context::*;
}

#[path = "routine_work_contract/support.rs"]
mod base_support;
#[path = "../src/cli/capture/mod.rs"]
mod capture;
#[path = "routine_recovery_reuse_journey_contract/catalog.rs"]
mod catalog;
#[path = "../src/digest.rs"]
mod digest;
#[path = "../src/fixture_scheduler/mod.rs"]
mod fixture_scheduler;
#[path = "routine_recovery_reuse_journey_contract/support.rs"]
mod journey_support;
#[path = "routine_recovery_reuse_journey_contract/journeys.rs"]
mod journeys;
#[path = "../src/orchestration/mod.rs"]
mod orchestration;
#[path = "../src/routine_work/mod.rs"]
mod routine_work;
