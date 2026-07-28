use super::super::{
    FixtureCaptureAdapter, ScheduledFixtureEvaluationBridge, ScheduledFixtureInvocation,
};
use crate::evaluation::runtime::ProductionExecutionRequest;
use crate::evaluation::{
    BoundInput, EvaluationSpec, EvaluationTask, EvaluationTaskDefinition, PerturbationControl,
    RuntimeConfiguration,
};
use crate::fixture_scheduler::{
    ExpectedOutcome, FixtureKind, FixtureScheduler, FixtureSpec, ResourceKind, RunDisposition,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::time::{SystemTime, UNIX_EPOCH};

fn root(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "hul-fixture-capture-{label}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn sha(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn fixture_spec(id: &str, expected: ExpectedOutcome) -> FixtureSpec {
    FixtureSpec::new(
        id,
        if matches!(
            &expected.verdict,
            crate::fixture_scheduler::OutcomeVerdict::Pass
        ) {
            FixtureKind::Positive
        } else {
            FixtureKind::Negative
        },
        "capture-execution",
        BTreeSet::from([ResourceKind::File, ResourceKind::Env, ResourceKind::Port]),
        expected,
        false,
    )
    .unwrap()
}

#[test]
fn pinned_fixture_adapter_executes_and_derives_acceptance() {
    let root = root("accept");
    let fixture = fixture_spec("executed", ExpectedOutcome::pass(2));
    let adapter = FixtureCaptureAdapter::issue(
        &fixture,
        "/usr/bin/true".into(),
        Vec::<OsString>::new(),
        4096,
        Vec::new(),
    )
    .unwrap();
    let mut scheduler = FixtureScheduler::new(&root);
    let lease = scheduler.schedule([fixture]).unwrap().pop().unwrap();
    let disposition = scheduler.execute(&lease, &adapter).unwrap();
    #[cfg(target_os = "freebsd")]
    assert_eq!(disposition, RunDisposition::Accepted);
    #[cfg(not(target_os = "freebsd"))]
    assert_eq!(disposition, RunDisposition::CleanupFailure);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn nonzero_pinned_fixture_derives_only_the_bound_causal_control() {
    let root = root("negative");
    let fixture = fixture_spec(
        "negative",
        ExpectedOutcome::causal_failure("intentional", 1),
    );
    let adapter = FixtureCaptureAdapter::issue(
        &fixture,
        "/usr/bin/false".into(),
        Vec::<OsString>::new(),
        4096,
        Vec::new(),
    )
    .unwrap();
    let mut scheduler = FixtureScheduler::new(&root);
    let lease = scheduler.schedule([fixture]).unwrap().pop().unwrap();
    let disposition = scheduler.execute(&lease, &adapter).unwrap();
    #[cfg(target_os = "freebsd")]
    assert_eq!(disposition, RunDisposition::CausalFailure);
    #[cfg(not(target_os = "freebsd"))]
    assert_eq!(disposition, RunDisposition::CleanupFailure);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn scheduled_evaluation_bridge_routes_through_sealed_production_request() {
    let root = root("evaluation");
    let input_root = root.join("inputs");
    let ledger_root = root.join("ledger");
    for directory in ["datasets", "scorers", "graders"] {
        std::fs::create_dir_all(input_root.join(directory)).unwrap();
    }
    let dataset = b"evaluation-dataset";
    let dataset_digest = digest(dataset);
    std::fs::write(input_root.join("datasets/core.json"), dataset).unwrap();
    let executable = root.join("evaluation-fixture.sh");
    let script = b"#!/bin/sh\nprintf '%s' 'accepted' > file/result.json\n";
    std::fs::write(&executable, script).unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let executable_digest = digest(script);
    let scorer_without_digest = json!({
        "schema_version":"EvaluationScorer-v2","dataset_digest_sha256":dataset_digest,
        "executable_digest_sha256":executable_digest,"primary_score_possible":10,
        "work_units":3,"failure_causal_code":"fixture-failed",
        "perturbation_controls":["verbosity","proof_artifact","receipt_production","test_manipulation","score_only"],
        "task_authority_id":"evaluation-test-authority","task_author_principal_id":"evaluation-test-author",
        "task_author_session_id":sha('1')
    });
    let scorer = serde_json::to_vec(&scorer_without_digest).unwrap();
    let scorer_digest = digest(&scorer);
    std::fs::write(input_root.join("scorers/scorer-core"), &scorer).unwrap();
    let grader = json!({
        "schema_version":"IndependentArtifactTextGrader-v2","dataset_digest_sha256":dataset_digest,
        "scorer_digest_sha256":scorer_digest,"root_authority_id":"evaluation-test-grader-authority",
        "independent_grader_principal_id":"evaluation-test-grader","independent_grader_session_id":sha('3'),
        "expected_artifact_text":"accepted","score_possible":10
    });
    std::fs::write(
        input_root.join("graders/core.json"),
        serde_json::to_vec(&grader).unwrap(),
    )
    .unwrap();
    let task = EvaluationTask::new(EvaluationTaskDefinition {
        task_id: "core".to_owned(),
        requirement_id: "REQ-core".to_owned(),
        behavior_id: "behavior-core".to_owned(),
        fixture_id: "fixture-core".to_owned(),
        dataset: BoundInput::regular("datasets/core.json", dataset_digest, dataset.len() as u64),
        scorer_id: "scorer-core".to_owned(),
        scorer_digest_sha256: scorer_digest,
        perturbation_controls: PerturbationControl::REQUIRED.into_iter().collect(),
        representative: true,
    });
    let spec = EvaluationSpec::new(sha('a'), sha('b'), "production-suite", vec![task]).unwrap();
    let mut bridge = ScheduledFixtureEvaluationBridge::new(
        &root,
        BTreeMap::from([(
            "fixture-core".to_owned(),
            ScheduledFixtureInvocation::new(
                executable,
                Vec::new(),
                16 * 1024,
                Vec::new(),
                "result.json",
            ),
        )]),
    );
    let request = ProductionExecutionRequest::new(
        &spec,
        &input_root,
        &ledger_root,
        [9; 32],
        sha('c'),
        sha('8'),
        RuntimeConfiguration::all_unknown(),
    );
    let result = request.execute(&mut bridge);
    assert!(
        result.as_ref().is_ok()
            || result
                .as_ref()
                .is_err_and(|error| { error.code() == "evaluation-fixture-recovery-required" })
    );
    let replay = ProductionExecutionRequest::new(
        &spec,
        &input_root,
        &ledger_root,
        [9; 32],
        sha('c'),
        sha('8'),
        RuntimeConfiguration::all_unknown(),
    )
    .execute(&mut bridge);
    assert_eq!(
        replay.unwrap_err().code(),
        "evaluation-production-execution-replayed"
    );
    assert!(bridge.recovery_required().len() <= 1);
    let _ = std::fs::remove_dir_all(root);
}
