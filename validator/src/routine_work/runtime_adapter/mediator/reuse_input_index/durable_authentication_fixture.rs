use super::super::*;
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
pub(super) struct DurableRecord {
    pub(super) fail_stage: AtomicBool,
    artifacts: Mutex<BTreeMap<String, String>>,
    pub(super) settlements: Mutex<Vec<DurableSettlement>>,
}

impl DurableAttemptAuthority for DurableRecord {
    fn validate_reserved(&self) -> Result<(), RoutineError> {
        Ok(())
    }

    fn stage_program(&self, _program: &PinnedExecutable) -> Result<StagedProgram, RoutineError> {
        Err(mediator_error("durable-authority-test-stage-unavailable"))
    }

    fn cleanup_staged(&self, _staged: &StagedProgram) -> Result<(), RoutineError> {
        Ok(())
    }

    fn prepare_spawn(&self) -> Result<(), RoutineError> {
        Ok(())
    }

    fn stage_success(&self, artifacts: &BTreeMap<String, String>) -> Result<(), RoutineError> {
        if self.fail_stage.load(Ordering::SeqCst) {
            return Err(mediator_error("durable-authority-test-stage-failed"));
        }
        self.artifacts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone_from(artifacts);
        Ok(())
    }

    fn settle(
        &self,
        outcome: DurableSettlement,
        _artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        self.settlements
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(outcome);
        Ok(())
    }

    fn authenticates_artifact(&self, digest: &str, witness: &str) -> Result<bool, RoutineError> {
        Ok(self
            .artifacts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(digest)
            .is_some_and(|expected| expected == witness))
    }

    fn recovery_is_durable(&self) -> bool {
        true
    }

    fn reuse_only(&self) -> bool {
        false
    }
}

pub(super) fn generated_artifact(label: &str) -> (String, String, Vec<u8>) {
    let value = sha256(format!("durable-authority-value-{label}").as_bytes());
    let witness = sha256(format!("durable-authority-witness-{label}").as_bytes());
    let result = json!({
        "schema_version": "RoutineMediatedResultArtifact-v2",
        "request_id": value,
        "protocol_id": value,
        "intent_id": value,
        "node_id": value,
        "behavior_id": "rust-source-syntax-v1",
        "plan_order": 0,
        "context_id": value,
        "candidate_id": value,
        "plan_id": value,
        "snapshot_id": value,
        "input_id": value,
        "tool_identity_sha256": value,
        "program_sha256": value,
        "environment_sha256": value,
        "read_authority_sha256": value,
        "dependency_results": {},
        "behavior_sha256": value,
        "output_files": {}
    });
    let bytes = serde_json::to_vec(&json!({
        "schema_version": "RoutineMediatedReuseArtifact-v2",
        "state": "complete",
        "protocol_id": value,
        "intent_id": value,
        "node_id": value,
        "behavior_id": "rust-source-syntax-v1",
        "plan_order": 0,
        "context_id": value,
        "candidate_id": value,
        "plan_id": value,
        "snapshot_id": value,
        "input_id": value,
        "tool_identity_sha256": value,
        "program_sha256": value,
        "environment_sha256": value,
        "read_authority_sha256": value,
        "dependency_results": {},
        "output_files": {},
        "result_artifact": result,
        "result_artifact_sha256": value,
        "mediator_witness_sha256": witness
    }))
    .unwrap();
    (sha256(&bytes), witness, bytes)
}
