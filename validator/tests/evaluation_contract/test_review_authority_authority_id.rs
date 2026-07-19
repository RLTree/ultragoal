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
    authority: &mut ReviewAuthorityHarness,
) -> PromotionReview {
    authority
        .issue_review(
            baseline,
            candidate,
            PromotionEvidencePaths {
                representative_journey: "src/evaluation/mod.rs".to_owned(),
                rollback_evidence: "src/evaluation/production_input.rs".to_owned(),
                reviewed_artifacts: vec!["tests/evaluation_contract/next_root.rs".to_owned()],
            },
        )
        .unwrap()
}

#[test]
fn opaque_promotion_review_debug_is_bounded_and_never_echoes_fields() {
    let baseline = run('1', BehaviorOutcome::Failed, 2);
    let candidate = run('2', BehaviorOutcome::Passed, 10);
    let mut authority = review_authority(&baseline, &candidate);
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
    assert!(audit
        .findings()
        .contains(&"evaluation-context-stale".to_owned()));
    assert!(audit
        .findings()
        .contains(&"evaluation-candidate-stale".to_owned()));
}

#[test]
fn missing_reward_hacking_control_blocks_audit() {
    let mut incomplete = controls();
    incomplete.remove(&PerturbationControl::ScoreOnly);
    let task = EvaluationTask::new(EvaluationTaskDefinition {
        task_id: "core".to_owned(),
        requirement_id: "REQ-core".to_owned(),
        behavior_id: "behavior-core".to_owned(),
        fixture_id: "fixture-core".to_owned(),
        dataset: BoundInput::regular("datasets/core.json", sha('a'), 32),
        scorer_id: "scorer-core".to_owned(),
        scorer_digest_sha256: sha('b'),
        perturbation_controls: incomplete,
        representative: true,
    });
    let spec = EvaluationSpec::new(sha('c'), sha('1'), "suite", vec![task]).unwrap();
    let audit = spec.audit(&sha('c'), &sha('1'));
    assert!(!audit.eligible());
    assert!(audit
        .findings()
        .iter()
        .any(|finding| finding.ends_with("score-only")));
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
        let task = EvaluationTask::new(EvaluationTaskDefinition {
            task_id: "core".to_owned(),
            requirement_id: "REQ-core".to_owned(),
            behavior_id: "behavior-core".to_owned(),
            fixture_id: "fixture-core".to_owned(),
            dataset: input,
            scorer_id: "scorer-core".to_owned(),
            scorer_digest_sha256: sha('b'),
            perturbation_controls: controls(),
            representative: true,
        });
        let spec = EvaluationSpec::new(sha('c'), sha('1'), "suite", vec![task]).unwrap();
        let audit = spec.audit(&sha('c'), &sha('1'));
        assert!(!audit.eligible());
        assert!(audit.findings().contains(&code.to_owned()), "{code:?}");
    }
}
