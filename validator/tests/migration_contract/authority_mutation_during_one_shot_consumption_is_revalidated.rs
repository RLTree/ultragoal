#[test]
fn authority_mutation_during_one_shot_consumption_is_revalidated() {
    let inventory = clean_inventory();
    let (plan, mut replacement_authority) = plan_with_authority(&inventory);
    let (review, mut review_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::PreservePhysicalArtifact,
        "retirement-reviewer",
    );
    review_authority.mutate_after_consume = true;
    let decision = RetirementDecision::reconcile_preservation(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &mut replacement_authority,
        &mut review_authority,
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"migration-retirement-review-invalid-or-replayed".to_owned())
    );

    let (plan, mut replacement_authority) = plan_with_authority(&inventory);
    let (review, mut review_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::RequestPhysicalDeletion,
        "retirement-reviewer",
    );
    let mut effect_authority =
        TestEffectAuthority::current(&inventory, review.effect_scope_sha256());
    let authorization = DestructiveAuthorization::issue(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &review_authority,
        &mut effect_authority,
    )
    .unwrap();
    effect_authority.mutate_after_consume = true;
    let decision = RetirementDecision::reconcile_destructive(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &authorization,
        &mut replacement_authority,
        &mut review_authority,
        &mut effect_authority,
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"migration-destructive-authorization-invalid-or-replayed".to_owned())
    );
}
