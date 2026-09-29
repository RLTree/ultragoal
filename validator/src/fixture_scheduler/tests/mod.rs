#[path = "../../../tests/fixture_scheduler_contract/adversarial/mod.rs"]
mod adversarial;
#[path = "../../../tests/fixture_scheduler_contract/confinement/mod.rs"]
mod confinement;
#[path = "../../../tests/fixture_scheduler_contract/isolation/mod.rs"]
mod isolation;
// FIXTURE-RECOVERY-STATE-CORRECTION-019 remains on disk as historical
// context. Its artifact bytes are stale, so it is not a live candidate gate.
#[path = "../../../tests/fixture_scheduler_contract/scheduling.rs"]
mod scheduling;
#[path = "../../../tests/fixture_scheduler_contract/worker_result/mod.rs"]
mod worker_result;
