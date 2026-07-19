use super::super::runtime::{FixtureEvaluationBridge, FixtureTaskRequest};
use super::super::*;
use super::custody::{execution_binding, root};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};

fn sha(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn preparation_spec(input_root: &std::path::Path) -> EvaluationSpec {
    for name in ["datasets", "scorers", "graders"] {
        fs::create_dir(input_root.join(name)).unwrap();
    }
    let dataset = br#"{\"case\":\"core\"}"#.to_vec();
    let dataset_digest = digest(&dataset);
    fs::write(input_root.join("datasets/core.json"), &dataset).unwrap();
    let controls = PerturbationControl::REQUIRED
        .into_iter()
        .collect::<BTreeSet<_>>();
    let scorer = serde_json::json!({
        "schema_version": "EvaluationScorer-v2", "dataset_digest_sha256": dataset_digest,
        "executable_digest_sha256": sha('8'), "primary_score_possible": 1, "work_units": 1,
        "failure_causal_code": "behavioral-failure", "perturbation_controls": controls,
        "task_authority_id": "evaluation-test-authority", "task_author_principal_id": "evaluation-test-author",
        "task_author_session_id": sha('1'),
    });
    let scorer = serde_json::to_vec(&scorer).unwrap();
    let scorer_digest = digest(&scorer);
    fs::write(input_root.join("scorers/scorer-core"), &scorer).unwrap();
    let grader = serde_json::json!({
        "schema_version": "IndependentArtifactTextGrader-v2", "dataset_digest_sha256": dataset_digest,
        "scorer_digest_sha256": scorer_digest, "root_authority_id": "evaluation-test-grader-authority",
        "independent_grader_principal_id": "evaluation-test-grader", "independent_grader_session_id": sha('3'),
        "expected_artifact_text": "pass", "score_possible": 1,
    });
    fs::write(
        input_root.join("graders/core.json"),
        serde_json::to_vec(&grader).unwrap(),
    )
    .unwrap();
    EvaluationSpec::new(
        sha('a'),
        sha('b'),
        "preparation",
        vec![EvaluationTask::new(EvaluationTaskDefinition {
            task_id: "core".to_owned(),
            requirement_id: "requirement-core".to_owned(),
            behavior_id: "behavior-core".to_owned(),
            fixture_id: "fixture-core".to_owned(),
            dataset: BoundInput::regular(
                "datasets/core.json",
                dataset_digest,
                dataset.len() as u64,
            ),
            scorer_id: "scorer-core".to_owned(),
            scorer_digest_sha256: scorer_digest,
            perturbation_controls: PerturbationControl::REQUIRED.into_iter().collect(),
            representative: true,
        })],
    )
    .unwrap()
}

struct RefusingBridge;

impl super::super::runtime::bridge_authority::Sealed for RefusingBridge {}

impl FixtureEvaluationBridge for RefusingBridge {
    fn execute_fixture(
        &mut self,
        _: &FixtureTaskRequest,
    ) -> Result<crate::fixture_scheduler::FixtureExecutionRecord, ProductionRuntimeError> {
        Err(ProductionRuntimeError::bridge(
            "evaluation-test-bridge-refused",
        ))
    }
}

#[test]
fn preparation_issues_private_authority_and_derives_material_binding() {
    let input_root = root("preparation-input");
    let ledger_root = root("preparation-ledger");
    let spec = preparation_spec(&input_root);
    let evidence = super::super::production_input::ProductionEvidenceRequest {
        task_authority_id: "evaluation-test-authority".to_owned(),
        task_principal_id: "evaluation-test-author".to_owned(),
        task_session_id: sha('1'),
        provenance_authority_id: "evaluation-test-provenance-authority".to_owned(),
        provenance_principal_id: "evaluation-test-provenance".to_owned(),
        provenance_session_id: sha('2'),
        grader_authority_id: "evaluation-test-grader-authority".to_owned(),
        grader_principal_id: "evaluation-test-grader".to_owned(),
        grader_session_id: sha('3'),
    };
    let request = super::super::runtime::ProductionExecutionRequest::new(
        &spec,
        &input_root,
        &ledger_root,
        [9; 32],
        evidence,
        sha('1'),
        sha('8'),
        RuntimeConfiguration::all_unknown(),
    );
    let error = request.execute(&mut RefusingBridge).unwrap_err();
    assert_eq!(error.code(), "evaluation-test-bridge-refused");
    assert!(fs::read_dir(&ledger_root).unwrap().next().is_some());
    fs::remove_dir_all(input_root).unwrap();
    fs::remove_dir_all(ledger_root).unwrap();
}

#[test]
fn execution_interruption_recovers_without_a_publication() {
    let root = root("interruption-recovery");
    let key = [7; 32];
    let binding = execution_binding('b', '1');
    let mut ledger =
        FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
    assert_eq!(
        ledger.reserve_outcome(&sha('7')).unwrap(),
        ExecutionReservationOutcome::Acquired
    );
    ledger
        .mark_interrupted("late-effect-revalidation-failed")
        .unwrap();
    drop(ledger);
    let mut reopened = FileEvaluationExecutionLedger::open(&root, key, binding).unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        EvaluationLedgerState::Interrupted { .. }
    ));
    reopened
        .require_recovery("late-effect-recovery-required")
        .unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        EvaluationLedgerState::RecoveryRequired { .. }
    ));
    reopened.reconcile_recovery(None).unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        EvaluationLedgerState::Initialized
    ));
    assert_eq!(
        reopened.reserve_outcome(&sha('8')).unwrap(),
        ExecutionReservationOutcome::Acquired
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn execution_symlink_and_lock_replacement_refuse_second_authority() {
    let root = root("descriptor-refusal");
    let key = [8; 32];
    let binding = execution_binding('b', '1');
    let mut ledger =
        FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
    let saved = root.with_extension("saved-lock");
    fs::rename(root.join("execution.lock"), &saved).unwrap();
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("execution.lock"))
        .unwrap();
    fs::set_permissions(
        root.join("execution.lock"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    assert!(ledger.reserve_outcome(&sha('7')).is_err());
    assert!(FileEvaluationExecutionLedger::open(&root, key, binding.clone()).is_err());
    fs::remove_file(root.join("execution.lock")).unwrap();
    fs::rename(&saved, root.join("execution.lock")).unwrap();
    fs::remove_file(root.join("execution.state")).unwrap();
    symlink("execution.anchor.journal", root.join("execution.state")).unwrap();
    assert!(FileEvaluationExecutionLedger::open(&root, key, binding).is_err());
    fs::remove_dir_all(root).unwrap();
}
