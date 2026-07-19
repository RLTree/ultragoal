use super::super::runtime::{FixtureEvaluationBridge, FixtureTaskRequest};
use super::super::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

static NEXT_PRODUCTION_ROOT: AtomicU64 = AtomicU64::new(0);
static PANIC_EFFECT_MARKER: Mutex<Option<std::path::PathBuf>> = Mutex::new(None);

pub(super) fn sha(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(super) fn production_root(label: &str) -> std::path::PathBuf {
    let root = std::path::PathBuf::from("/private/tmp").join(format!(
        "hul-evaluation-panic-{label}-{}-{}",
        std::process::id(),
        NEXT_PRODUCTION_ROOT.fetch_add(1, Ordering::SeqCst),
    ));
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    root
}

pub(super) fn preparation_spec(input_root: &std::path::Path) -> EvaluationSpec {
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

pub(super) struct RefusingBridge;
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

pub(super) struct PanickingBridge;
impl super::super::runtime::bridge_authority::Sealed for PanickingBridge {}
impl FixtureEvaluationBridge for PanickingBridge {
    fn execute_fixture(
        &mut self,
        _: &FixtureTaskRequest,
    ) -> Result<crate::fixture_scheduler::FixtureExecutionRecord, ProductionRuntimeError> {
        if let Some(marker) = PANIC_EFFECT_MARKER.lock().unwrap().as_ref() {
            fs::write(marker, b"fixture-effect-observed-before-panic").unwrap();
        }
        panic!("fixture bridge panic must become an interrupted ledger state");
    }
}

pub(super) fn set_panic_effect_marker(marker: Option<std::path::PathBuf>) {
    *PANIC_EFFECT_MARKER.lock().unwrap() = marker;
}
