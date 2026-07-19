use super::super::production_input::ProductionSpecPermit;
use super::super::runtime::{self, FixtureEvaluationBridge, FixtureTaskRequest};
use super::super::*;
use super::custody::root;
use crate::fixture_scheduler::{
    FixtureExecutionRecord, FixtureExecutionRecordCapture, ObservedOutcome,
};
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;

fn digest(bytes: &[u8]) -> String {
    super::super::digest(bytes)
}

fn controls() -> BTreeSet<PerturbationControl> {
    PerturbationControl::REQUIRED.into_iter().collect()
}

fn production_spec(root: &std::path::Path) -> EvaluationSpec {
    fs::create_dir(root.join("datasets")).unwrap();
    fs::create_dir(root.join("scorers")).unwrap();
    fs::create_dir(root.join("graders")).unwrap();
    let dataset = b"stable evaluation dataset";
    fs::write(root.join("datasets/core.json"), dataset).unwrap();
    let dataset_digest = digest(dataset);
    let scorer = json!({
        "schema_version": "EvaluationScorer-v2", "dataset_digest_sha256": dataset_digest,
        "executable_digest_sha256": format!("sha256:{}", "1".repeat(64)),
        "primary_score_possible": 10, "work_units": 3, "failure_causal_code": "fixture-failed",
        "perturbation_controls": ["verbosity", "proof_artifact", "receipt_production", "test_manipulation", "score_only"],
        "task_authority_id": "evaluation-test-authority", "task_author_principal_id": "evaluation-test-author",
        "task_author_session_id": format!("sha256:{}", "1".repeat(64)),
    });
    let scorer = serde_json::to_vec(&scorer).unwrap();
    fs::write(root.join("scorers/scorer-core"), &scorer).unwrap();
    let scorer_digest = digest(&scorer);
    fs::write(root.join("graders/core.json"), serde_json::to_vec(&json!({
        "schema_version": "IndependentArtifactTextGrader-v2", "dataset_digest_sha256": dataset_digest,
        "scorer_digest_sha256": scorer_digest, "root_authority_id": "evaluation-test-grader-authority",
        "independent_grader_principal_id": "evaluation-test-grader", "independent_grader_session_id": format!("sha256:{}", "3".repeat(64)),
        "expected_artifact_text": "accepted", "score_possible": 10,
    })).unwrap()).unwrap();
    EvaluationSpec::new(
        format!("sha256:{}", "a".repeat(64)),
        format!("sha256:{}", "b".repeat(64)),
        "production-suite",
        vec![EvaluationTask::new(EvaluationTaskDefinition {
            task_id: "core".to_owned(),
            requirement_id: "REQ-core".to_owned(),
            behavior_id: "behavior-core".to_owned(),
            fixture_id: "fixture-core".to_owned(),
            dataset: BoundInput::regular(
                "datasets/core.json",
                dataset_digest,
                dataset.len() as u64,
            ),
            scorer_id: "scorer-core".to_owned(),
            scorer_digest_sha256: scorer_digest,
            perturbation_controls: controls(),
            representative: true,
        })],
    )
    .unwrap()
}

struct LateSwapBridge {
    dataset: std::path::PathBuf,
}
impl runtime::bridge_authority::Sealed for LateSwapBridge {}
impl FixtureEvaluationBridge for LateSwapBridge {
    fn execute_fixture(
        &mut self,
        request: &FixtureTaskRequest,
    ) -> Result<FixtureExecutionRecord, ProductionRuntimeError> {
        fs::rename(&self.dataset, self.dataset.with_extension("saved")).unwrap();
        fs::write(&self.dataset, b"substituted dataset").unwrap();
        Ok(FixtureExecutionRecord::captured(
            FixtureExecutionRecordCapture {
                binding: request.binding.clone(),
                fixture_id: request.fixture_id.clone(),
                fixture_digest_sha256: format!("sha256:{}", "f".repeat(64)),
                lease_id: "lease-core".to_owned(),
                executable_digest_sha256: request.executable_digest_sha256.clone(),
                executable_identity_sha256: format!("sha256:{}", "2".repeat(64)),
                artifact_relative_path: "result.json".to_owned(),
                artifact_bytes: b"accepted".to_vec(),
                outcome: ObservedOutcome::pass(0),
                exit_code: Some(0),
                stdout_digest_sha256: format!("sha256:{}", "3".repeat(64)),
                stderr_digest_sha256: format!("sha256:{}", "4".repeat(64)),
                output_redacted: false,
            },
        ))
    }
}

#[test]
fn late_input_swap_after_fixture_effect_settles_execution_as_interrupted() {
    let input_root = root("runtime-late-input");
    let ledger_root = root("runtime-late-input-ledger");
    let spec = production_spec(&input_root);
    let permit = ProductionSpecPermit::test_issue(&spec, &input_root).unwrap();
    let mut ledger = FileEvaluationExecutionLedger::initialize(
        &ledger_root,
        [9; 32],
        EvaluationExecutionBinding::new(EvaluationExecutionBindingRequest {
            live_context_id: spec.live_context_id().to_owned(),
            candidate_id: spec.candidate_id().to_owned(),
            spec_sha256: spec.spec_sha256().to_owned(),
            task_set_sha256: spec.task_set_sha256().to_owned(),
            execution_session_id: format!("sha256:{}", "7".repeat(64)),
            execution_material_set_sha256: permit.material_set_sha256().to_owned(),
            artifact_root_sha256: format!("sha256:{}", "8".repeat(64)),
        })
        .unwrap(),
    )
    .unwrap();
    let error = runtime::execute_production(
        &permit,
        format!("sha256:{}", "7".repeat(64)),
        RuntimeConfiguration::all_unknown(),
        &mut ledger,
        &mut LateSwapBridge {
            dataset: input_root.join("datasets/core.json"),
        },
    )
    .unwrap_err();
    assert_eq!(error.code(), "evaluation-production-input-changed");
    assert!(
        matches!(ledger.inspect().unwrap(), EvaluationLedgerState::Interrupted { causal_code } if causal_code == "evaluation-production-input-changed")
    );
    fs::remove_dir_all(input_root).unwrap();
    fs::remove_dir_all(ledger_root).unwrap();
}
