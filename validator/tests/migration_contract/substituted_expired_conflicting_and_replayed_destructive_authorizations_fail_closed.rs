#[test]
fn substituted_expired_conflicting_and_replayed_destructive_authorizations_fail_closed() {
    let inventory = clean_inventory();
    let (plan, mut replacement_authority) = plan_with_authority(&inventory);

    let (review, mut review_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::RequestPhysicalDeletion,
        "retirement-reviewer",
    );
    let mut effect_authority =
        TestEffectAuthority::current(&inventory, review.effect_scope_sha256());
    let mut substituted = DestructiveAuthorization::issue(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &review_authority,
        &mut effect_authority,
    )
    .unwrap();
    substituted.substitute_effect_scope_for_test(sha('3'));
    let decision = RetirementDecision::reconcile_destructive(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &substituted,
        &mut replacement_authority,
        &mut review_authority,
        &mut effect_authority,
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"migration-destructive-authorization-invalid".to_owned())
    );

    let (expired_review, mut expired_review_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::RequestPhysicalDeletion,
        "retirement-reviewer",
    );
    let mut expired_effect =
        TestEffectAuthority::current(&inventory, expired_review.effect_scope_sha256());
    let expired_authorization = DestructiveAuthorization::issue(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &expired_review,
        &expired_review_authority,
        &mut expired_effect,
    )
    .unwrap();
    expired_effect.now = expired_effect.expires_at + 1;
    let expired = RetirementDecision::reconcile_destructive(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &expired_review,
        &expired_authorization,
        &mut replacement_authority,
        &mut expired_review_authority,
        &mut expired_effect,
    );
    assert_eq!(expired.status, RetirementStatus::Blocked);

    let (replay_review, mut replay_review_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::RequestPhysicalDeletion,
        "retirement-reviewer",
    );
    let mut replay_effect =
        TestEffectAuthority::current(&inventory, replay_review.effect_scope_sha256());
    let replay_authorization = DestructiveAuthorization::issue(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &replay_review,
        &replay_review_authority,
        &mut replay_effect,
    )
    .unwrap();
    assert_eq!(
        RetirementDecision::reconcile_destructive(
            &plan,
            "retire-LEGACY-SKILL:old",
            &inventory,
            &replay_review,
            &replay_authorization,
            &mut replacement_authority,
            &mut replay_review_authority,
            &mut replay_effect,
        )
        .status,
        RetirementStatus::DestructiveRetirementCandidate
    );
    let replay = RetirementDecision::reconcile_destructive(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &replay_review,
        &replay_authorization,
        &mut replacement_authority,
        &mut replay_review_authority,
        &mut replay_effect,
    );
    assert_eq!(replay.status, RetirementStatus::Blocked);
    assert!(
        replay
            .reasons
            .contains(&"migration-replacement-ledger-invalid-or-replayed".to_owned())
    );

    let (deletion_review, deletion_review_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::RequestPhysicalDeletion,
        "retirement-reviewer",
    );
    let mut conflicting_effect =
        TestEffectAuthority::current(&inventory, deletion_review.effect_scope_sha256());
    let conflicting_authorization = DestructiveAuthorization::issue(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &deletion_review,
        &deletion_review_authority,
        &mut conflicting_effect,
    )
    .unwrap();
    let (preserve_review, mut preserve_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::PreservePhysicalArtifact,
        "second-retirement-reviewer",
    );
    let conflict = RetirementDecision::reconcile_destructive(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &preserve_review,
        &conflicting_authorization,
        &mut replacement_authority,
        &mut preserve_authority,
        &mut conflicting_effect,
    );
    assert!(
        conflict
            .reasons
            .contains(&"migration-conflicting-destructive-authorization".to_owned())
    );
}
