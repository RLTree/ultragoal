#![allow(dead_code)]

#[path = "../src/evaluation/mod.rs"]
mod evaluation;
#[path = "../src/fixture_scheduler/mod.rs"]
mod fixture_scheduler;

use evaluation::{
    BehaviorOutcome, BoundInput, CapturedTaskObservation, EvaluationError, EvaluationExecutor,
    EvaluationRun, EvaluationSpec, EvaluationTask, FailureCase, InputKind, PerturbationControl,
    PromotionDecision, PromotionReview, PromotionReviewAuthority, PromotionStatus, TaskAudit,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

fn require_type<T>() {}

#[test]
fn required_evaluation_api_types_compile_in_the_leased_module() {
    require_type::<EvaluationSpec>();
    require_type::<TaskAudit>();
    require_type::<EvaluationRun>();
    require_type::<FailureCase>();
    require_type::<PromotionDecision>();
}

fn sha(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn controls() -> BTreeSet<PerturbationControl> {
    PerturbationControl::REQUIRED.into_iter().collect()
}

fn task(id: &str, representative: bool) -> EvaluationTask {
    let dataset_digest = match id {
        "core" => sha('a'),
        "recovery" => sha('0'),
        _ => sha('f'),
    };
    EvaluationTask::new(
        id,
        format!("REQ-{id}"),
        format!("behavior-{id}"),
        format!("fixture-{id}"),
        BoundInput::regular(format!("datasets/{id}.json"), dataset_digest, 128),
        format!("scorer-{id}"),
        sha('b'),
        controls(),
        representative,
    )
}

fn spec(candidate: char) -> EvaluationSpec {
    EvaluationSpec::new(
        sha('c'),
        sha(candidate),
        "representative-suite",
        vec![task("core", true), task("recovery", true)],
    )
    .unwrap()
}

#[derive(Clone)]
struct ScriptedExecutor {
    context: String,
    candidate: String,
    session: String,
    rows: BTreeMap<String, CapturedTaskObservation>,
    drift_after_first: bool,
    drift_session_after_first: bool,
    calls: usize,
}

impl ScriptedExecutor {
    fn new(candidate: char, core: BehaviorOutcome, core_score: u64) -> Self {
        let mut rows = BTreeMap::new();
        rows.insert(
            "core".to_owned(),
            observation("core", core, core_score, 'd'),
        );
        rows.insert(
            "recovery".to_owned(),
            observation("recovery", BehaviorOutcome::Passed, 10, 'e'),
        );
        Self {
            context: sha('c'),
            candidate: sha(candidate),
            session: execution_session(candidate),
            rows,
            drift_after_first: false,
            drift_session_after_first: false,
            calls: 0,
        }
    }
}

impl EvaluationExecutor for ScriptedExecutor {
    fn binding(&self) -> (&str, &str) {
        (&self.context, &self.candidate)
    }

    fn session_id(&self) -> &str {
        &self.session
    }

    fn execute(
        &mut self,
        task: &EvaluationTask,
    ) -> Result<CapturedTaskObservation, EvaluationError> {
        self.calls += 1;
        let row = self.rows.get(task.task_id()).unwrap().clone();
        if self.drift_after_first && self.calls == 1 {
            self.candidate = sha('f');
        }
        if self.drift_session_after_first && self.calls == 1 {
            self.session = sha('f');
        }
        Ok(row)
    }
}

fn execution_session(candidate: char) -> String {
    match candidate {
        '1' => sha('4'),
        '2' => sha('5'),
        _ => sha('6'),
    }
}

fn observation(
    id: &str,
    outcome: BehaviorOutcome,
    score: u64,
    digest_byte: char,
) -> CapturedTaskObservation {
    let causal = if outcome == BehaviorOutcome::Passed {
        "behavioral-pass"
    } else {
        "causal-product-failure"
    };
    CapturedTaskObservation::captured(
        id,
        format!("fixture-{id}"),
        outcome,
        causal,
        score,
        10,
        3,
        BoundInput::regular(format!("artifacts/{id}.json"), sha(digest_byte), 96),
        sha(digest_byte),
        format!("executor-{id}"),
        format!("observer-{id}"),
        format!("grader-{id}"),
        score,
        10,
        controls(),
    )
}

fn run(candidate: char, core: BehaviorOutcome, score: u64) -> EvaluationRun {
    let spec = spec(candidate);
    let audit = spec.audit(&sha('c'), &sha(candidate));
    let mut executor = ScriptedExecutor::new(candidate, core, score);
    EvaluationRun::execute_local(&spec, &audit, &mut executor).unwrap()
}

struct TestReviewAuthority {
    authority_id: String,
    reviewer_id: String,
    session_id: String,
    context_id: String,
    candidate_id: String,
    secret: String,
    consumed: BTreeSet<String>,
}

impl TestReviewAuthority {
    fn current(candidate: char) -> Self {
        Self {
            authority_id: "root-review-authority".to_owned(),
            reviewer_id: "independent-reviewer".to_owned(),
            session_id: sha('7'),
            context_id: sha('c'),
            candidate_id: sha(candidate),
            secret: "test-only-sealed-review-key".to_owned(),
            consumed: BTreeSet::new(),
        }
    }

    fn attestation(&self, binding_sha256: &str, reviewer_id: &str) -> String {
        test_digest(
            format!(
                "{}|{}|{}|{}",
                self.secret, self.authority_id, binding_sha256, reviewer_id
            )
            .as_bytes(),
        )
    }
}

impl PromotionReviewAuthority for TestReviewAuthority {
    fn authority_id(&self) -> &str {
        &self.authority_id
    }

    fn reviewer_id(&self) -> &str {
        &self.reviewer_id
    }

    fn review_session_id(&self) -> &str {
        &self.session_id
    }

    fn current_binding(&self) -> (&str, &str) {
        (&self.context_id, &self.candidate_id)
    }

    fn issue_attestation(&mut self, binding_sha256: &str) -> Result<String, EvaluationError> {
        Ok(self.attestation(binding_sha256, &self.reviewer_id))
    }

    fn verify_and_consume(
        &mut self,
        binding_sha256: &str,
        reviewer_id: &str,
        review_id: &str,
        attestation_sha256: &str,
    ) -> bool {
        let expected_attestation = self.attestation(binding_sha256, reviewer_id);
        let expected_review_id = test_digest(
            format!("promotion-review|{binding_sha256}|{expected_attestation}").as_bytes(),
        );
        reviewer_id == self.reviewer_id
            && expected_attestation == attestation_sha256
            && expected_review_id == review_id
            && self.consumed.insert(review_id.to_owned())
    }
}

fn test_digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn review_evidence() -> (BoundInput, BoundInput, Vec<BoundInput>) {
    (
        BoundInput::regular("journeys/representative.json", sha('8'), 128),
        BoundInput::regular("rollback/verified.json", sha('9'), 128),
        vec![BoundInput::regular(
            "validator/src/evaluation/mod.rs",
            sha('6'),
            256,
        )],
    )
}

fn review(
    baseline: &EvaluationRun,
    candidate: &EvaluationRun,
    authority: &mut TestReviewAuthority,
) -> PromotionReview {
    let (journey, rollback, artifacts) = review_evidence();
    PromotionReview::issue(baseline, candidate, journey, rollback, artifacts, authority).unwrap()
}

#[test]
fn opaque_promotion_review_debug_is_bounded_and_never_echoes_fields() {
    let baseline = run('1', BehaviorOutcome::Failed, 2);
    let candidate = run('2', BehaviorOutcome::Passed, 10);
    let mut authority = TestReviewAuthority::current('2');
    let mut review = review(&baseline, &candidate, &mut authority);
    let sentinels = review.inject_debug_sentinels_for_test();

    let debug = format!("{review:?}");
    assert_eq!(debug, "PromotionReview { contents: \"<redacted>\" }");
    assert!(debug.len() <= 64, "opaque Debug output is unbounded");
    for sentinel in sentinels {
        assert!(
            !debug.contains(&sentinel),
            "opaque Debug echoed a private field: {sentinel}"
        );
    }
}

#[test]
fn valid_spec_audits_deterministically() {
    let spec = spec('1');
    let first = spec.audit(&sha('c'), &sha('1'));
    let second = spec.audit(&sha('c'), &sha('1'));
    assert!(first.eligible());
    assert_eq!(first, second);
    assert!(first.findings().is_empty());
}

#[test]
fn stale_context_and_candidate_are_refused() {
    let spec = spec('1');
    let audit = spec.audit(&sha('2'), &sha('3'));
    assert!(!audit.eligible());
    assert!(
        audit
            .findings()
            .contains(&"evaluation-context-stale".to_owned())
    );
    assert!(
        audit
            .findings()
            .contains(&"evaluation-candidate-stale".to_owned())
    );
}

#[test]
fn missing_reward_hacking_control_blocks_audit() {
    let mut incomplete = controls();
    incomplete.remove(&PerturbationControl::ScoreOnly);
    let task = EvaluationTask::new(
        "core",
        "REQ-core",
        "behavior-core",
        "fixture-core",
        BoundInput::regular("datasets/core.json", sha('a'), 32),
        "scorer-core",
        sha('b'),
        incomplete,
        true,
    );
    let spec = EvaluationSpec::new(sha('c'), sha('1'), "suite", vec![task]).unwrap();
    let audit = spec.audit(&sha('c'), &sha('1'));
    assert!(!audit.eligible());
    assert!(
        audit
            .findings()
            .iter()
            .any(|finding| finding.ends_with("score-only"))
    );
}

#[test]
fn unsafe_dataset_metadata_fails_without_opening_it() {
    for (input, code) in [
        (
            BoundInput::observed("../secret", sha('a'), 10, 1, InputKind::Regular),
            "evaluation-dataset-unsafe-path",
        ),
        (
            BoundInput::observed("data/a", sha('a'), 10, 1, InputKind::Symlink),
            "evaluation-dataset-non-regular-input",
        ),
        (
            BoundInput::observed("data/a", sha('a'), 10, 2, InputKind::Regular),
            "evaluation-dataset-hardlink-rejected",
        ),
        (
            BoundInput::observed("data/a", sha('a'), 10, 1, InputKind::Special),
            "evaluation-dataset-non-regular-input",
        ),
        (
            BoundInput::observed("data/a", sha('a'), u64::MAX, 1, InputKind::Regular),
            "evaluation-dataset-size-out-of-bounds",
        ),
    ] {
        let task = EvaluationTask::new(
            "core",
            "REQ-core",
            "behavior-core",
            "fixture-core",
            input,
            "scorer-core",
            sha('b'),
            controls(),
            true,
        );
        let spec = EvaluationSpec::new(sha('c'), sha('1'), "suite", vec![task]).unwrap();
        let audit = spec.audit(&sha('c'), &sha('1'));
        assert!(!audit.eligible());
        assert!(audit.findings().contains(&code.to_owned()), "{code:?}");
    }
}

#[test]
fn known_training_overlap_is_a_contamination_finding() {
    let dataset = BoundInput::regular("datasets/core.json", sha('a'), 32);
    let task = EvaluationTask::new_with_provenance(
        "core",
        "REQ-core",
        "behavior-core",
        "fixture-core",
        dataset,
        sha('7'),
        BTreeSet::from([sha('a')]),
        "scorer-core",
        sha('b'),
        controls(),
        true,
    );
    let spec = EvaluationSpec::new(sha('c'), sha('1'), "suite", vec![task]).unwrap();
    let audit = spec.audit(&sha('c'), &sha('1'));
    assert!(!audit.eligible());
    assert!(
        audit
            .findings()
            .contains(&"evaluation-dataset-contamination-detected".to_owned())
    );
}

#[test]
fn crate_controlled_execution_produces_stable_vendor_neutral_runs() {
    let first = run('1', BehaviorOutcome::Failed, 3);
    let second = run('1', BehaviorOutcome::Failed, 3);
    assert_eq!(first.run_sha256(), second.run_sha256());
    assert_eq!(first.results().len(), 2);
    assert_eq!(first.candidate_id(), sha('1'));
}

#[test]
fn failure_harvest_contains_only_observed_causal_failures() {
    let run = run('1', BehaviorOutcome::Failed, 3);
    let failures = run.harvest_failures();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].task_id, "core");
    assert_eq!(failures[0].causal_code, "causal-product-failure");
    assert_eq!(failures[0].candidate_id, sha('1'));
}

#[test]
fn zero_work_and_self_observation_are_refused() {
    let spec = spec('1');
    let audit = spec.audit(&sha('c'), &sha('1'));
    let mut executor = ScriptedExecutor::new('1', BehaviorOutcome::Failed, 3);
    executor.rows.insert(
        "core".to_owned(),
        CapturedTaskObservation::captured(
            "core",
            "fixture-core",
            BehaviorOutcome::Passed,
            "behavioral-pass",
            10,
            10,
            0,
            BoundInput::regular("artifacts/core.json", sha('d'), 20),
            sha('d'),
            "same-actor",
            "same-actor",
            "grader-core",
            10,
            10,
            controls(),
        ),
    );
    assert_eq!(
        EvaluationRun::execute_local(&spec, &audit, &mut executor)
            .unwrap_err()
            .code(),
        "evaluation-observation-refused"
    );
}

#[test]
fn post_execution_candidate_drift_invalidates_the_run() {
    let spec = spec('1');
    let audit = spec.audit(&sha('c'), &sha('1'));
    let mut executor = ScriptedExecutor::new('1', BehaviorOutcome::Failed, 3);
    executor.drift_after_first = true;
    assert_eq!(
        EvaluationRun::execute_local(&spec, &audit, &mut executor)
            .unwrap_err()
            .code(),
        "evaluation-input-drift"
    );
}

#[test]
fn post_execution_session_drift_invalidates_the_run() {
    let spec = spec('1');
    let audit = spec.audit(&sha('c'), &sha('1'));
    let mut executor = ScriptedExecutor::new('1', BehaviorOutcome::Failed, 3);
    executor.drift_session_after_first = true;
    assert_eq!(
        EvaluationRun::execute_local(&spec, &audit, &mut executor)
            .unwrap_err()
            .code(),
        "evaluation-input-drift"
    );
}

#[test]
fn replay_nondeterminism_and_grader_disagreement_are_refused() {
    for (replay, independent_score) in [(sha('9'), 10), (sha('d'), 9)] {
        let spec = spec('1');
        let audit = spec.audit(&sha('c'), &sha('1'));
        let mut executor = ScriptedExecutor::new('1', BehaviorOutcome::Failed, 3);
        executor.rows.insert(
            "core".to_owned(),
            CapturedTaskObservation::captured(
                "core",
                "fixture-core",
                BehaviorOutcome::Failed,
                "causal-product-failure",
                3,
                10,
                1,
                BoundInput::regular("artifacts/core.json", sha('d'), 32),
                replay,
                "executor-core",
                "observer-core",
                "grader-core",
                independent_score,
                10,
                controls(),
            ),
        );
        assert_eq!(
            EvaluationRun::execute_local(&spec, &audit, &mut executor)
                .unwrap_err()
                .code(),
            "evaluation-observation-refused"
        );
    }
}

#[test]
fn mutate_restore_requires_a_fresh_audit() {
    let mut spec = spec('1');
    let audit = spec.audit(&sha('c'), &sha('1'));
    spec.corrupt_candidate_for_test(sha('2'));
    spec.corrupt_candidate_for_test(sha('1'));
    let mut executor = ScriptedExecutor::new('1', BehaviorOutcome::Failed, 3);
    assert_eq!(
        EvaluationRun::execute_local(&spec, &audit, &mut executor)
            .unwrap_err()
            .code(),
        "evaluation-audit-stale-or-ineligible"
    );
}

#[test]
fn paired_behavior_improvement_yields_only_an_improvement_candidate() {
    let baseline = run('1', BehaviorOutcome::Failed, 2);
    let candidate = run('2', BehaviorOutcome::Passed, 10);
    let mut authority = TestReviewAuthority::current('2');
    let review = review(&baseline, &candidate, &mut authority);
    let decision = PromotionDecision::reconcile(&baseline, &candidate, &review, &mut authority);
    assert_eq!(decision.status, PromotionStatus::ImprovementCandidate);
    assert_eq!(
        decision.claim_ceiling,
        "improvement_candidate_not_product_completion"
    );
    assert!(decision.reasons.is_empty());
}

#[test]
fn score_gain_without_behavior_improvement_is_a_false_pass() {
    let baseline = run('1', BehaviorOutcome::Passed, 8);
    let candidate = run('2', BehaviorOutcome::Passed, 10);
    let mut authority = TestReviewAuthority::current('2');
    let review = review(&baseline, &candidate, &mut authority);
    let decision = PromotionDecision::reconcile(&baseline, &candidate, &review, &mut authority);
    assert_eq!(decision.status, PromotionStatus::Rejected);
    assert!(
        decision
            .reasons
            .contains(&"evaluation-no-representative-behavior-improvement".to_owned())
    );
}

#[test]
fn self_review_cannot_promote() {
    let baseline = run('1', BehaviorOutcome::Failed, 2);
    let candidate = run('2', BehaviorOutcome::Passed, 10);
    let mut authority = TestReviewAuthority::current('2');
    authority.reviewer_id = "observer-core".to_owned();
    let (journey, rollback, artifacts) = review_evidence();
    let error = PromotionReview::issue(
        &baseline,
        &candidate,
        journey,
        rollback,
        artifacts,
        &mut authority,
    )
    .unwrap_err();
    assert_eq!(error.code(), "evaluation-review-issuance-refused");
}

#[test]
fn promotion_requires_journey_rollback_and_bounded_change() {
    let baseline = run('1', BehaviorOutcome::Failed, 2);
    let candidate = run('2', BehaviorOutcome::Passed, 10);
    for (journey, rollback, artifacts) in [
        (
            BoundInput::regular("journeys/current.json", "missing", 20),
            BoundInput::regular("rollback/current.json", sha('9'), 20),
            vec![BoundInput::regular("validator/src/lib.rs", sha('6'), 20)],
        ),
        (
            BoundInput::regular("journeys/current.json", sha('8'), 20),
            BoundInput::regular("rollback/current.json", "missing", 20),
            vec![BoundInput::regular("validator/src/lib.rs", sha('6'), 20)],
        ),
        (
            BoundInput::regular("journeys/current.json", sha('8'), 20),
            BoundInput::regular("rollback/current.json", sha('9'), 20),
            vec![BoundInput::regular("../escape", sha('6'), 20)],
        ),
    ] {
        let mut authority = TestReviewAuthority::current('2');
        let error = PromotionReview::issue(
            &baseline,
            &candidate,
            journey,
            rollback,
            artifacts,
            &mut authority,
        )
        .unwrap_err();
        assert_eq!(error.code(), "evaluation-review-issuance-refused");
    }
}

#[test]
fn review_source_principal_or_execution_session_cannot_issue() {
    let baseline = run('1', BehaviorOutcome::Failed, 2);
    let candidate = run('2', BehaviorOutcome::Passed, 10);
    for (authority_id, session_id) in [
        ("grader-core", sha('7')),
        ("independent-reviewer", sha('7')),
        ("root-review-authority", execution_session('1')),
        ("root-review-authority", execution_session('2')),
    ] {
        let mut authority = TestReviewAuthority::current('2');
        authority.authority_id = authority_id.to_owned();
        authority.session_id = session_id;
        let (journey, rollback, artifacts) = review_evidence();
        assert_eq!(
            PromotionReview::issue(
                &baseline,
                &candidate,
                journey,
                rollback,
                artifacts,
                &mut authority,
            )
            .unwrap_err()
            .code(),
            "evaluation-review-issuance-refused"
        );
    }
}

#[test]
fn stale_authority_and_substituted_review_bindings_are_rejected() {
    let baseline = run('1', BehaviorOutcome::Failed, 2);
    let candidate = run('2', BehaviorOutcome::Passed, 10);

    let mut stale_authority = TestReviewAuthority::current('2');
    let stale_review = review(&baseline, &candidate, &mut stale_authority);
    stale_authority.candidate_id = sha('3');
    let stale =
        PromotionDecision::reconcile(&baseline, &candidate, &stale_review, &mut stale_authority);
    assert_eq!(stale.status, PromotionStatus::Rejected);
    assert!(
        stale
            .reasons
            .contains(&"evaluation-review-authority-stale".to_owned())
    );

    let mut substituted_authority = TestReviewAuthority::current('2');
    let mut substituted = review(&baseline, &candidate, &mut substituted_authority);
    substituted.substitute_candidate_for_test(sha('3'));
    let decision = PromotionDecision::reconcile(
        &baseline,
        &candidate,
        &substituted,
        &mut substituted_authority,
    );
    assert_eq!(decision.status, PromotionStatus::Rejected);
    assert!(
        decision
            .reasons
            .contains(&"evaluation-review-binding-stale-or-substituted".to_owned())
    );

    let mut run_authority = TestReviewAuthority::current('2');
    let run_review = review(&baseline, &candidate, &mut run_authority);
    let substituted_run = run('2', BehaviorOutcome::Passed, 9);
    let decision =
        PromotionDecision::reconcile(&baseline, &substituted_run, &run_review, &mut run_authority);
    assert_eq!(decision.status, PromotionStatus::Rejected);
    assert!(
        decision
            .reasons
            .contains(&"evaluation-review-binding-stale-or-substituted".to_owned())
    );

    let mut spec_authority = TestReviewAuthority::current('2');
    let mut substituted_spec = review(&baseline, &candidate, &mut spec_authority);
    substituted_spec.substitute_candidate_spec_for_test(sha('0'));
    let decision = PromotionDecision::reconcile(
        &baseline,
        &candidate,
        &substituted_spec,
        &mut spec_authority,
    );
    assert_eq!(decision.status, PromotionStatus::Rejected);
    assert!(
        decision
            .reasons
            .contains(&"evaluation-review-binding-stale-or-substituted".to_owned())
    );

    let mut evidence_authority = TestReviewAuthority::current('2');
    let mut substituted_evidence = review(&baseline, &candidate, &mut evidence_authority);
    substituted_evidence.substitute_journey_for_test(BoundInput::regular(
        "journeys/substituted.json",
        sha('0'),
        128,
    ));
    let decision = PromotionDecision::reconcile(
        &baseline,
        &candidate,
        &substituted_evidence,
        &mut evidence_authority,
    );
    assert_eq!(decision.status, PromotionStatus::Rejected);
    assert!(
        decision
            .reasons
            .contains(&"evaluation-review-binding-stale-or-substituted".to_owned())
    );
}

#[test]
fn review_attestation_is_consumed_once_and_replay_is_rejected() {
    let baseline = run('1', BehaviorOutcome::Failed, 2);
    let candidate = run('2', BehaviorOutcome::Passed, 10);
    let mut authority = TestReviewAuthority::current('2');
    let review = review(&baseline, &candidate, &mut authority);
    assert_eq!(
        PromotionDecision::reconcile(&baseline, &candidate, &review, &mut authority).status,
        PromotionStatus::ImprovementCandidate
    );
    let replay = PromotionDecision::reconcile(&baseline, &candidate, &review, &mut authority);
    assert_eq!(replay.status, PromotionStatus::Rejected);
    assert!(
        replay
            .reasons
            .contains(&"evaluation-review-attestation-invalid-or-replayed".to_owned())
    );
}

#[test]
fn missing_observed_reward_control_blocks_run_not_just_promotion() {
    let spec = spec('1');
    let audit = spec.audit(&sha('c'), &sha('1'));
    let mut executor = ScriptedExecutor::new('1', BehaviorOutcome::Failed, 2);
    let mut incomplete = controls();
    incomplete.remove(&PerturbationControl::ReceiptProduction);
    executor.rows.insert(
        "core".to_owned(),
        CapturedTaskObservation::captured(
            "core",
            "fixture-core",
            BehaviorOutcome::Failed,
            "causal-product-failure",
            2,
            10,
            1,
            BoundInput::regular("artifacts/core.json", sha('d'), 32),
            sha('d'),
            "executor-core",
            "observer-core",
            "grader-core",
            2,
            10,
            incomplete,
        ),
    );
    assert_eq!(
        EvaluationRun::execute_local(&spec, &audit, &mut executor)
            .unwrap_err()
            .code(),
        "evaluation-observation-refused"
    );
}

#[test]
fn audit_and_reconciliation_are_zero_write() {
    let root = temp_root("zero-write");
    fs::write(root.join("sentinel"), b"preserve").unwrap();
    let before = tree(&root);
    let spec = spec('1');
    let _ = spec.audit(&sha('c'), &sha('1'));
    let baseline = run('1', BehaviorOutcome::Failed, 2);
    let candidate = run('2', BehaviorOutcome::Passed, 10);
    let mut authority = TestReviewAuthority::current('2');
    let review = review(&baseline, &candidate, &mut authority);
    let _ = PromotionDecision::reconcile(&baseline, &candidate, &review, &mut authority);
    assert_eq!(tree(&root), before);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn external_callers_cannot_mint_a_review_with_an_independence_boolean() {
    let source =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/evaluation/mod.rs"))
            .unwrap();
    let implementation = source
        .split("impl PromotionReview {")
        .nth(1)
        .and_then(|source| source.split("\n}\n").next())
        .expect("PromotionReview implementation remains present");
    assert!(!implementation.contains("pub fn new"));
    assert!(!implementation.contains("pub fn issue"));
    assert!(implementation.contains("pub(crate) fn issue"));
    assert!(!implementation.contains("independent: bool"));
}

#[test]
fn fixture_catalog_names_every_false_pass_family() {
    let paired: Value = serde_json::from_str(include_str!(
        "../../fixtures/evaluation-engine/paired-valid.json"
    ))
    .unwrap();
    let red: Value = serde_json::from_str(include_str!(
        "../../fixtures/evaluation-engine/red-cases.json"
    ))
    .unwrap();
    assert_eq!(paired["schema_version"], "EvaluationEngineFixture-v1");
    let cases = red["cases"].as_array().unwrap();
    assert!(cases.len() >= 24);
    for expected in [
        "metric-gain-without-behavior",
        "verbosity-reward",
        "proof-artifact-reward",
        "receipt-production-reward",
        "test-manipulation-reward",
        "score-only-promotion",
        "caller-asserted-independent-boolean",
        "same-review-source-principal",
        "same-review-session",
        "stale-review-binding",
        "substituted-journey-evidence",
        "review-attestation-replay",
    ] {
        assert!(cases.iter().any(|case| case == expected));
    }
}

fn temp_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "hul-evaluation-043-{label}-{}-{}",
        std::process::id(),
        NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    fn walk(root: &Path, current: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(current).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    walk(root, root, &mut out);
    out
}
