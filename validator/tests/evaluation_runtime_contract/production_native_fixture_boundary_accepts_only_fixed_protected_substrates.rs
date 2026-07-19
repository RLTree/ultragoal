use crate::evaluation::runtime::{
    FixtureEvaluationBridge, FixtureTaskRequest, ProductionExecutionRequest,
};
use crate::evaluation::{
    BoundInput, ConfigurationExposure, EvaluationSpec, EvaluationTask, EvaluationTaskDefinition,
    PerturbationControl, ProductionRuntimeError, RuntimeConfiguration,
};
use crate::fixture_scheduler::{
    FixtureExecutionRecord, FixtureExecutionRecordCapture, ObservedOutcome,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

fn sha(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn controls() -> BTreeSet<PerturbationControl> {
    PerturbationControl::REQUIRED.into_iter().collect()
}

fn root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "hul-evaluation-runtime-{label}-{}-{}",
        std::process::id(),
        NEXT_ROOT.fetch_add(1, Ordering::SeqCst),
    ));
    fs::create_dir_all(root.join("datasets")).unwrap();
    fs::create_dir_all(root.join("scorers")).unwrap();
    fs::create_dir_all(root.join("graders")).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    root
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn production_spec(root: &Path, candidate: char) -> EvaluationSpec {
    let dataset = b"evaluation-contract-dataset";
    let dataset_digest = digest(dataset);
    fs::write(root.join("datasets/core.json"), dataset).unwrap();
    let scorer = serde_json::to_vec(&json!({
        "schema_version": "EvaluationScorer-v2", "dataset_digest_sha256": dataset_digest,
        "executable_digest_sha256": sha('1'), "primary_score_possible": 10, "work_units": 3,
        "failure_causal_code": "fixture-failed", "perturbation_controls": [
            "verbosity", "proof_artifact", "receipt_production", "test_manipulation", "score_only"
        ], "task_authority_id": "evaluation-test-authority", "task_author_principal_id": "evaluation-test-author",
        "task_author_session_id": sha('2')
    }))
    .unwrap();
    let scorer_digest = digest(&scorer);
    fs::write(root.join("scorers/scorer-core"), &scorer).unwrap();
    fs::write(root.join("graders/core.json"), serde_json::to_vec(&json!({
        "schema_version": "IndependentArtifactTextGrader-v2", "dataset_digest_sha256": dataset_digest,
        "scorer_digest_sha256": scorer_digest, "root_authority_id": "evaluation-test-grader-authority",
        "independent_grader_principal_id": "evaluation-test-grader", "independent_grader_session_id": sha('3'),
        "expected_artifact_text": "accepted", "score_possible": 10
    })).unwrap()).unwrap();
    EvaluationSpec::new(
        sha('a'),
        sha(candidate),
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

pub(super) fn task(id: &str, dataset: char) -> EvaluationTask {
    EvaluationTask::new(EvaluationTaskDefinition {
        task_id: id.to_owned(),
        requirement_id: format!("REQ-{id}"),
        behavior_id: format!("behavior-{id}"),
        fixture_id: format!("fixture-{id}"),
        dataset: BoundInput::regular(format!("datasets/{id}.json"), sha(dataset), 128),
        scorer_id: format!("scorer-{id}"),
        scorer_digest_sha256: sha('e'),
        perturbation_controls: controls(),
        representative: true,
    })
}

struct FakeConfinedBridge;

impl crate::evaluation::runtime::bridge_authority::Sealed for FakeConfinedBridge {}

impl FixtureEvaluationBridge for FakeConfinedBridge {
    fn execute_fixture(
        &mut self,
        request: &FixtureTaskRequest,
    ) -> Result<FixtureExecutionRecord, ProductionRuntimeError> {
        Ok(FixtureExecutionRecord::captured(
            FixtureExecutionRecordCapture {
                binding: request.binding.clone(),
                fixture_id: request.fixture_id.clone(),
                fixture_digest_sha256: sha('f'),
                lease_id: "lease-core".to_owned(),
                executable_digest_sha256: sha('1'),
                executable_identity_sha256: sha('2'),
                artifact_relative_path: "result.json".to_owned(),
                artifact_bytes: b"accepted".to_vec(),
                outcome: ObservedOutcome::pass(0),
                exit_code: Some(0),
                stdout_digest_sha256: sha('3'),
                stderr_digest_sha256: sha('4'),
                output_redacted: false,
            },
        ))
    }
}

#[test]
fn audited_runtime_is_stable_artifact_bearing_and_explicitly_unknown() {
    let input_root = root("stable-input");
    let spec = production_spec(&input_root, 'b');
    let first_ledger = input_root.join("ledger-first");
    let mut first_bridge = FakeConfinedBridge;
    let first = ProductionExecutionRequest::new(
        &spec,
        &input_root,
        &first_ledger,
        [5; 32],
        sha('5'),
        sha('8'),
        RuntimeConfiguration::all_unknown(),
    )
    .execute(&mut first_bridge)
    .unwrap();
    let second_ledger = input_root.join("ledger-second");
    let mut second_bridge = FakeConfinedBridge;
    let second = ProductionExecutionRequest::new(
        &spec,
        &input_root,
        &second_ledger,
        [5; 32],
        sha('5'),
        sha('8'),
        RuntimeConfiguration::all_unknown(),
    )
    .execute(&mut second_bridge)
    .unwrap();
    assert_eq!(first.canonical_run, second.canonical_run);
    assert_eq!(first.fixture_records().len(), 1);
    assert!(first.fixture_records()[0].artifact_byte_length > 0);
    assert_eq!(first.canonical_failures, Vec::new());
    assert_eq!(
        first.canonical_run.runtime_configuration.model,
        ConfigurationExposure::Unknown
    );
    assert_eq!(
        RuntimeConfiguration::from_prompt_text("model=gpt-from-prompt")
            .unwrap_err()
            .code(),
        "evaluation-prompt-derived-runtime-metadata-refused"
    );
    fs::remove_dir_all(input_root).unwrap();
}

#[test]
fn privacy_projection_has_no_claim_authority_or_secret_surface() {
    let input_root = root("privacy-input");
    let spec = production_spec(&input_root, 'b');
    let ledger_root = input_root.join("ledger");
    let mut bridge = FakeConfinedBridge;
    let run = ProductionExecutionRequest::new(
        &spec,
        &input_root,
        &ledger_root,
        [5; 32],
        sha('5'),
        sha('8'),
        RuntimeConfiguration::all_unknown(),
    )
    .execute(&mut bridge)
    .unwrap();
    let json = serde_json::to_string(&run.events)
        .unwrap()
        .to_ascii_lowercase();
    for forbidden in [
        "claim_id",
        "claim_authority",
        "readiness",
        "release",
        "acceptance",
        "completion",
        "api_key",
        "bearer ",
    ] {
        assert!(!json.contains(forbidden), "leaked event field {forbidden}");
    }
    fs::remove_dir_all(input_root).unwrap();
}

pub(super) const RESEARCH_CHECKED_DAY_EPOCH: u64 = 1_783_900_800;
pub(super) const RESEARCH_2026_01_01_EPOCH: u64 = 1_767_225_600;
pub(super) const RESEARCH_MUTABLE_FRESHNESS_SECONDS: u64 = 30 * 86_400;

pub(super) fn research_epoch(offset_seconds: u64) -> u64 {
    RESEARCH_CHECKED_DAY_EPOCH + offset_seconds
}

pub(super) fn research_laws() -> BTreeSet<String> {
    BTreeSet::from(["HUL-RESEARCH-001".to_owned()])
}
