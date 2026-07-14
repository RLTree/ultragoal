#[path = "../src/digest.rs"]
mod digest;
#[path = "../src/fixture_scheduler/mod.rs"]
mod fixture_scheduler;
#[path = "../src/orchestration/mod.rs"]
mod orchestration;

#[path = "fixture_scheduler_contract/adversarial/mod.rs"]
mod adversarial;
#[path = "fixture_scheduler_contract/confinement/mod.rs"]
mod confinement;
#[path = "fixture_scheduler_contract/execution_adapter/mod.rs"]
mod execution_adapter;
#[path = "fixture_scheduler_contract/isolation/mod.rs"]
mod isolation;
// FIXTURE-RECOVERY-STATE-CORRECTION-019 is retained on disk as historical
// context. Its artifact bytes are no longer current, so its live-workspace
// self-check must not remain an active gate for later scheduler candidates.
#[path = "fixture_scheduler_contract/scheduling.rs"]
mod scheduling;
#[path = "fixture_scheduler_contract/worker_result/mod.rs"]
mod worker_result;
