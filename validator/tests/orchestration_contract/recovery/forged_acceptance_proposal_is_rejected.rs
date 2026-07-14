#[test]
fn forged_acceptance_proposal_is_rejected() {
    let mut engine = started_engine();
    let (_, _, result) = retry_subject();
    let review = review_for_result(&result, ReviewDecision::Pass);
    engine.submit_structural(3, "lease-001", &result).unwrap();
    engine
        .assign_review(4, "lease-001", worker("reviewer-a"))
        .unwrap();
    engine
        .record_review(5, "lease-001", review.clone())
        .unwrap();
    let forged = AcceptanceProposal {
        schema_version: "AcceptanceProposal-v1".to_owned(),
        binding: binding(),
        node_id: "node-a".to_owned(),
        lease_id: "lease-001".to_owned(),
        result_id: digest('e'),
        result_commitment_id: digest('f'),
        review_id: review.review_id().unwrap(),
        artifact_digests: Default::default(),
        expected_root_changes: Default::default(),
        requested_root_change_count: 0,
        requested_root_changes_digest:
            "sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a".to_owned(),
        root_decision_required: true,
    };
    assert_eq!(
        engine.accept(6, &forged).unwrap_err(),
        OrchestrationError::InvalidReview
    );
}

#[test]
fn malformed_review_payload_cannot_enter_replay_log() {
    let mut engine = started_engine();
    let (_, _, result) = retry_subject();
    engine.submit_structural(3, "lease-001", &result).unwrap();
    engine
        .assign_review(4, "lease-001", worker("reviewer-a"))
        .unwrap();
    let mut forged = review_for_result(&result, ReviewDecision::Pass);
    forged.worker = "different-worker".to_owned();
    let count = engine.events().len();
    assert_eq!(
        engine.record_review(5, "lease-001", forged).unwrap_err(),
        OrchestrationError::ReviewerNotIndependent
    );
    assert_eq!(engine.events().len(), count);
}

#[test]
fn recomputed_attacker_root_change_commitment_is_not_the_reviewed_result() {
    let mut engine = started_engine();
    let (lease, package, result) = retry_subject();
    let review = review_for_result(&result, ReviewDecision::Pass);
    let mut proposal = propose_acceptance(&binding(), &package, &lease, &result, &review).unwrap();
    engine.submit_structural(3, "lease-001", &result).unwrap();
    engine
        .assign_review(4, "lease-001", worker("reviewer-a"))
        .unwrap();
    engine.record_review(5, "lease-001", review).unwrap();
    *proposal.expected_root_changes.values_mut().next().unwrap() = digest('7');
    proposal.requested_root_change_count = proposal.expected_root_changes.len();
    proposal.requested_root_changes_digest = proposal.computed_root_changes_digest().unwrap();
    proposal.result_commitment_id = proposal.computed_result_commitment_id().unwrap();
    proposal.validate().unwrap();
    let before_log = engine.event_log();
    let before_plan = engine.plan().unwrap();
    assert_eq!(
        engine.accept(6, &proposal).unwrap_err(),
        OrchestrationError::InvalidReview
    );
    assert_eq!(engine.event_log(), before_log);
    assert_eq!(engine.plan().unwrap(), before_plan);
}
