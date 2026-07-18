use crate::evaluation::runtime::{FixtureEvaluationBridge, FixtureTaskRequest, execute_production};
use crate::evaluation::{
    BoundInput, ConfigurationExposure, EvaluationSpec, EvaluationTask, EvaluationTaskDefinition,
    PerturbationControl, ProductionRuntimeError, RuntimeConfiguration,
};
use crate::fixture_scheduler::{
    FixtureExecutionRecord, FixtureExecutionRecordCapture, ObservedOutcome,
};
use serde_json::json;
use std::collections::BTreeSet;

fn sha(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn controls() -> BTreeSet<PerturbationControl> {
    PerturbationControl::REQUIRED.into_iter().collect()
}

fn task(id: &str, dataset: char) -> EvaluationTask {
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

fn spec(candidate: char) -> EvaluationSpec {
    EvaluationSpec::new(
        sha('a'),
        sha(candidate),
        "production-suite",
        vec![task("core", 'd')],
    )
    .unwrap()
}

struct FakeConfinedBridge;

impl FixtureEvaluationBridge for FakeConfinedBridge {
    fn execute_fixture(
        &mut self,
        request: &FixtureTaskRequest,
    ) -> Result<FixtureExecutionRecord, ProductionRuntimeError> {
        let artifact = serde_json::to_vec(&json!({
            "schema_version": "EvaluationFixtureArtifact-v1",
            "task_id": request.binding.task_id,
            "fixture_id": request.fixture_id,
            "outcome": "passed",
            "causal_code": "behavioral-pass",
            "score_earned": 10,
            "score_possible": 10,
            "work_units": 3,
            "producer_id": "fixture-producer",
            "observer_id": "fixture-observer",
            "independent_grader_id": "independent-grader",
            "independent_score_earned": 10,
            "independent_score_possible": 10,
            "passed_perturbations": [
                "verbosity",
                "proof_artifact",
                "receipt_production",
                "test_manipulation",
                "score_only"
            ]
        }))
        .unwrap();
        Ok(FixtureExecutionRecord::captured(
            FixtureExecutionRecordCapture {
                binding: request.binding.clone(),
                fixture_id: request.fixture_id.clone(),
                fixture_digest_sha256: sha('f'),
                lease_id: "lease-core".to_owned(),
                executable_digest_sha256: sha('1'),
                executable_identity_sha256: sha('2'),
                artifact_relative_path: "result.json".to_owned(),
                artifact_bytes: artifact,
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
    let spec = spec('b');
    let audit = spec.audit(&sha('a'), &sha('b'));
    let mut first_bridge = FakeConfinedBridge;
    let first = execute_production(
        &spec,
        &audit,
        sha('5'),
        RuntimeConfiguration::all_unknown(),
        &mut first_bridge,
    )
    .unwrap();
    let mut second_bridge = FakeConfinedBridge;
    let second = execute_production(
        &spec,
        &audit,
        sha('5'),
        RuntimeConfiguration::all_unknown(),
        &mut second_bridge,
    )
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
}

#[test]
fn privacy_projection_has_no_claim_authority_or_secret_surface() {
    let spec = spec('b');
    let audit = spec.audit(&sha('a'), &sha('b'));
    let mut bridge = FakeConfinedBridge;
    let run = execute_production(
        &spec,
        &audit,
        sha('5'),
        RuntimeConfiguration::all_unknown(),
        &mut bridge,
    )
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
}

const RESEARCH_CHECKED_DAY_EPOCH: u64 = 1_783_900_800;
const RESEARCH_2026_01_01_EPOCH: u64 = 1_767_225_600;
const RESEARCH_MUTABLE_FRESHNESS_SECONDS: u64 = 30 * 86_400;

fn research_epoch(offset_seconds: u64) -> u64 {
    RESEARCH_CHECKED_DAY_EPOCH + offset_seconds
}

fn research_laws() -> BTreeSet<String> {
    BTreeSet::from(["HUL-RESEARCH-001".to_owned()])
}
