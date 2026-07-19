use super::super::production_input::ProductionSpecPermit;
use super::super::runtime::{FixtureEvaluationBridge, FixtureTaskRequest};
use super::super::*;
use super::custody::{execution_binding, root};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_PRODUCTION_ROOT: AtomicU64 = AtomicU64::new(0);

fn sha(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn production_root(label: &str) -> std::path::PathBuf {
    let root = std::path::PathBuf::from("/private/tmp").join(format!(
        "hul-evaluation-panic-{label}-{}-{}",
        std::process::id(),
        NEXT_PRODUCTION_ROOT.fetch_add(1, Ordering::SeqCst),
    ));
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    root
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

struct PanickingBridge;

impl super::super::runtime::bridge_authority::Sealed for PanickingBridge {}

impl FixtureEvaluationBridge for PanickingBridge {
    fn execute_fixture(
        &mut self,
        _: &FixtureTaskRequest,
    ) -> Result<crate::fixture_scheduler::FixtureExecutionRecord, ProductionRuntimeError> {
        panic!("fixture bridge panic must become an interrupted ledger state");
    }
}

#[test]
fn preparation_issues_private_authority_and_derives_material_binding() {
    let input_root = root("preparation-input");
    let ledger_root = root("preparation-ledger");
    let spec = preparation_spec(&input_root);
    let request = super::super::runtime::ProductionExecutionRequest::new(
        &spec,
        &input_root,
        &ledger_root,
        [9; 32],
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
fn execution_panic_is_interrupted_and_requires_explicit_recovery() {
    let input_root = production_root("input");
    let ledger_root = production_root("ledger");
    let spec = preparation_spec(&input_root);
    let error = super::super::runtime::ProductionExecutionRequest::new(
        &spec,
        &input_root,
        &ledger_root,
        [6; 32],
        sha('6'),
        sha('8'),
        RuntimeConfiguration::all_unknown(),
    )
    .execute(&mut PanickingBridge)
    .unwrap_err();
    assert_eq!(error.code(), "evaluation-execution-panicked");

    let binding = EvaluationExecutionBinding::new(EvaluationExecutionBindingRequest {
        live_context_id: spec.live_context_id().to_owned(),
        candidate_id: spec.candidate_id().to_owned(),
        spec_sha256: spec.spec_sha256().to_owned(),
        task_set_sha256: spec.task_set_sha256().to_owned(),
        execution_session_id: sha('6'),
        execution_material_set_sha256: ProductionSpecPermit::test_issue(&spec, &input_root)
            .unwrap()
            .material_set_sha256()
            .to_owned(),
        artifact_root_sha256: sha('8'),
    })
    .unwrap();
    let mut ledger = FileEvaluationExecutionLedger::open(&ledger_root, [6; 32], binding).unwrap();
    assert!(matches!(
        ledger.inspect().unwrap(),
        EvaluationLedgerState::Interrupted { causal_code }
            if causal_code == "evaluation-execution-panicked"
    ));
    ledger
        .require_recovery("evaluation-panic-recovery-required")
        .unwrap();
    ledger.reconcile_recovery(None).unwrap();
    assert!(matches!(
        ledger.inspect().unwrap(),
        EvaluationLedgerState::Initialized
    ));
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
