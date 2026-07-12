#[path = "../src/digest.rs"]
mod digest;
#[path = "../src/fixture_scheduler/mod.rs"]
mod fixture_scheduler;
#[path = "../src/orchestration/mod.rs"]
mod orchestration;

#[path = "fixture_scheduler_contract/adversarial.rs"]
mod adversarial;
#[path = "fixture_scheduler_contract/confinement.rs"]
mod confinement;
#[path = "fixture_scheduler_contract/execution_adapter.rs"]
mod execution_adapter;
#[path = "fixture_scheduler_contract/isolation.rs"]
mod isolation;
#[path = "fixture_scheduler_contract/lease_identity_worker_result.rs"]
mod lease_identity_worker_result;
#[path = "fixture_scheduler_contract/scheduling.rs"]
mod scheduling;
#[path = "fixture_scheduler_contract/worker_result.rs"]
mod worker_result;
