#[test]
fn self_review_cannot_promote() {
    let baseline = run('1', BehaviorOutcome::Failed, 2);
    let candidate = run('2', BehaviorOutcome::Passed, 10);
    let mut authority = ReviewAuthorityHarness::with_ids(
        &baseline,
        &candidate,
        "root-review-authority",
        "observer-core",
        sha('7'),
    )
    .unwrap();
    let (journey, rollback, artifacts) = review_evidence();
    let error = PromotionReview::issue(
        &baseline,
        &candidate,
        PromotionReviewEvidence {
            representative_journey: journey,
            rollback_evidence: rollback,
            reviewed_artifacts: artifacts,
        },
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
        let mut authority = review_authority(&baseline, &candidate);
        let error = PromotionReview::issue(
            &baseline,
            &candidate,
            PromotionReviewEvidence {
                representative_journey: journey,
                rollback_evidence: rollback,
                reviewed_artifacts: artifacts,
            },
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
        let authority = ReviewAuthorityHarness::with_ids(
            &baseline,
            &candidate,
            authority_id,
            "independent-reviewer",
            session_id.clone(),
        );
        let Ok(mut authority) = authority else {
            assert!(
                authority_id == "independent-reviewer"
                    || session_id == execution_session('1')
                    || session_id == execution_session('2')
            );
            continue;
        };
        let (journey, rollback, artifacts) = review_evidence();
        assert_eq!(
            PromotionReview::issue(
                &baseline,
                &candidate,
                PromotionReviewEvidence {
                    representative_journey: journey,
                    rollback_evidence: rollback,
                    reviewed_artifacts: artifacts,
                },
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

    let mut issuing_authority = review_authority(&baseline, &candidate);
    let stale_review = review(&baseline, &candidate, &mut issuing_authority);
    let stale_candidate = run('3', BehaviorOutcome::Passed, 10);
    let mut stale_authority = review_authority(&baseline, &stale_candidate);
    let stale =
        PromotionDecision::reconcile(&baseline, &candidate, &stale_review, &mut stale_authority);
    assert_eq!(stale.status, PromotionStatus::Rejected);
    assert!(
        stale
            .reasons
            .contains(&"evaluation-review-authority-stale".to_owned())
    );

    let mut substituted_authority = review_authority(&baseline, &candidate);
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

    let mut run_authority = review_authority(&baseline, &candidate);
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

    let mut spec_authority = review_authority(&baseline, &candidate);
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

    let mut evidence_authority = review_authority(&baseline, &candidate);
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
    let mut authority = review_authority(&baseline, &candidate);
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
