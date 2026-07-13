#![allow(dead_code, unused_imports)]

mod context {
    pub use ultragoal::context::*;
}

#[path = "../src/cli/capture/mod.rs"]
mod capture;

#[path = "../src/routine_work/mod.rs"]
mod routine_work;

#[path = "routine_work_contract/contract.rs"]
mod contract;
#[path = "routine_work_contract/local_capture.rs"]
mod local_capture;
#[path = "routine_work_contract/planning.rs"]
mod planning;
#[path = "routine_work_contract/provenance.rs"]
mod provenance;
#[path = "routine_work_contract/report.rs"]
mod report;
#[path = "routine_work_contract/reuse/mod.rs"]
mod reuse;
#[path = "routine_work_contract/runtime_adapter.rs"]
mod runtime_adapter;
#[path = "routine_work_contract/runtime_mediator.rs"]
mod runtime_mediator;
#[path = "routine_work_contract/security.rs"]
mod security;
#[path = "routine_work_contract/support.rs"]
mod support;
