#[test]
fn retirement_review_issuance_rejects_same_principal_session_expiry_and_stale_binding() {
    let inventory = clean_inventory();
    let (plan, _replacement_authority) = plan_with_authority(&inventory);
    for case in 0..6 {
        let mut authority =
            TestReviewAuthority::current(&inventory, Od009Decision::PreservePhysicalArtifact);
        match case {
            0 => authority.authority_id = authority.reviewer_id.clone(),
            1 => authority.session_id = sha('f'),
            2 => authority.now = authority.expires_at + 1,
            3 => authority.inventory_sha256 = sha('0'),
            4 => authority.authority_id = "migration-owner".to_owned(),
            5 => authority.nonce_sha256 = "caller-nonce".to_owned(),
            _ => unreachable!(),
        }
        assert_review_issue_refused(&plan, &inventory, authority);
    }
}

#[test]
fn stale_substituted_unknown_and_replayed_retirement_reviews_fail_closed() {
    let inventory = clean_inventory();
    let (plan, mut replacement_authority) = plan_with_authority(&inventory);

    let (mut substituted_plan, mut plan_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::PreservePhysicalArtifact,
        "retirement-reviewer",
    );
    substituted_plan.substitute_plan_for_test(sha('0'));
    let decision = RetirementDecision::reconcile_preservation(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &substituted_plan,
        &mut replacement_authority,
        &mut plan_authority,
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"migration-retirement-review-invalid".to_owned())
    );

    let (mut substituted_target, mut target_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::PreservePhysicalArtifact,
        "retirement-reviewer",
    );
    substituted_target.substitute_target_for_test("retire-UNKNOWN");
    let decision = RetirementDecision::reconcile_preservation(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &substituted_target,
        &mut replacement_authority,
        &mut target_authority,
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);

    let (stale_review, mut stale_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::PreservePhysicalArtifact,
        "retirement-reviewer",
    );
    stale_authority.candidate_id = sha('0');
    let decision = RetirementDecision::reconcile_preservation(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &stale_review,
        &mut replacement_authority,
        &mut stale_authority,
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);

    let (expired_review, mut expired_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::PreservePhysicalArtifact,
        "retirement-reviewer",
    );
    expired_authority.now = expired_authority.expires_at + 1;
    let expired = RetirementDecision::reconcile_preservation(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &expired_review,
        &mut replacement_authority,
        &mut expired_authority,
    );
    assert_eq!(expired.status, RetirementStatus::Blocked);

    let (unknown_review, mut unknown_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::PreservePhysicalArtifact,
        "retirement-reviewer",
    );
    let unknown = RetirementDecision::reconcile_preservation(
        &plan,
        "retire-UNKNOWN",
        &inventory,
        &unknown_review,
        &mut replacement_authority,
        &mut unknown_authority,
    );
    assert!(
        unknown
            .reasons
            .contains(&"migration-retirement-target-unknown".to_owned())
    );

    let (review, mut authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::PreservePhysicalArtifact,
        "retirement-reviewer",
    );
    assert_eq!(
        RetirementDecision::reconcile_preservation(
            &plan,
            "retire-LEGACY-SKILL:old",
            &inventory,
            &review,
            &mut replacement_authority,
            &mut authority,
        )
        .status,
        RetirementStatus::NonAuthoritativePreservationCandidate
    );
    let replay = RetirementDecision::reconcile_preservation(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &mut replacement_authority,
        &mut authority,
    );
    assert_eq!(replay.status, RetirementStatus::Blocked);
    assert!(
        replay
            .reasons
            .contains(&"migration-replacement-ledger-invalid-or-replayed".to_owned())
    );
}
