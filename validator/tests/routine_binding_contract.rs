#![allow(dead_code, unused_imports)]

mod context {
    pub use ultragoal::context::*;
}

#[path = "../src/cli/capture/mod.rs"]
mod capture;
#[path = "routine_binding_contract/closed_binding.rs"]
mod closed_binding;
#[path = "routine_binding_contract/execution_fixture.rs"]
mod execution_fixture;
#[path = "../src/routine_work/mod.rs"]
mod routine_work;
#[path = "routine_work_contract/scenario.rs"]
mod scenario;
