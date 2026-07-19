#[test]
fn known_training_overlap_is_a_contamination_finding() {
    let dataset = BoundInput::regular("datasets/core.json", sha('a'), 32);
    let task = EvaluationTask::new_with_provenance(
        EvaluationTaskDefinition {
            task_id: "core".to_owned(),
            requirement_id: "REQ-core".to_owned(),
            behavior_id: "behavior-core".to_owned(),
            fixture_id: "fixture-core".to_owned(),
            dataset,
            scorer_id: "scorer-core".to_owned(),
            scorer_digest_sha256: sha('b'),
            perturbation_controls: controls(),
            representative: true,
        },
        EvaluationDatasetProvenance {
            dataset_provenance_sha256: sha('7'),
            known_training_corpus_sha256s: BTreeSet::from([sha('a')]),
        },
    );
    let spec = EvaluationSpec::new(sha('c'), sha('1'), "suite", vec![task]).unwrap();
    let audit = spec.audit(&sha('c'), &sha('1'));
    assert!(!audit.eligible());
    assert!(audit
        .findings()
        .contains(&"evaluation-dataset-contamination-detected".to_owned()));
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
        CapturedTaskObservation::captured(CapturedTaskObservationRecord {
            task_id: "core".to_owned(),
            fixture_id: "fixture-core".to_owned(),
            outcome: BehaviorOutcome::Passed,
            causal_code: "behavioral-pass".to_owned(),
            score_earned: 10,
            score_possible: 10,
            work_units: 0,
            artifact: BoundInput::regular("artifacts/core.json", sha('d'), 20),
            replay_artifact_digest_sha256: sha('d'),
            producer_id: "same-actor".to_owned(),
            observer_id: "same-actor".to_owned(),
            independent_grader_id: "grader-core".to_owned(),
            independent_score_earned: 10,
            independent_score_possible: 10,
            passed_perturbations: controls(),
        }),
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
            CapturedTaskObservation::captured(CapturedTaskObservationRecord {
                task_id: "core".to_owned(),
                fixture_id: "fixture-core".to_owned(),
                outcome: BehaviorOutcome::Failed,
                causal_code: "causal-product-failure".to_owned(),
                score_earned: 3,
                score_possible: 10,
                work_units: 1,
                artifact: BoundInput::regular("artifacts/core.json", sha('d'), 32),
                replay_artifact_digest_sha256: replay,
                producer_id: "executor-core".to_owned(),
                observer_id: "observer-core".to_owned(),
                independent_grader_id: "grader-core".to_owned(),
                independent_score_earned: independent_score,
                independent_score_possible: 10,
                passed_perturbations: controls(),
            }),
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
    let mut authority = review_authority(&baseline, &candidate);
    let review = review(&baseline, &candidate, &mut authority);
    let decision = PromotionDecision::reconcile(&baseline, &candidate, &review, &mut authority);
    assert_eq!(decision.status, PromotionStatus::ImprovementCandidate);
    assert_eq!(
        decision.claim_ceiling,
        "improvement_candidate_not_product_completion"
    );
    assert!(decision.reasons.is_empty());
    authority.teardown().unwrap();
}

#[test]
fn score_gain_without_behavior_improvement_is_a_false_pass() {
    let baseline = run('1', BehaviorOutcome::Passed, 8);
    let candidate = run('2', BehaviorOutcome::Passed, 10);
    let mut authority = review_authority(&baseline, &candidate);
    let review = review(&baseline, &candidate, &mut authority);
    let decision = PromotionDecision::reconcile(&baseline, &candidate, &review, &mut authority);
    assert_eq!(decision.status, PromotionStatus::Rejected);
    assert!(decision
        .reasons
        .contains(&"evaluation-no-representative-behavior-improvement".to_owned()));
    authority.teardown().unwrap();
}
