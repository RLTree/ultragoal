#[test]
fn replacement_reviewer_and_issuer_must_be_independent_of_route_owner() {
    let inventory = clean_inventory();
    let route = route("LEGACY-SKILL:old", "SKILL:current");
    for issuer_is_owner in [false, true] {
        let mut authority = TestReplacementAuthority::current(&inventory, &route);
        if issuer_is_owner {
            authority.authority_id = "migration-owner".to_owned();
        } else {
            authority.reviewer_id = "migration-owner".to_owned();
        }
        assert_eq!(
            ReplacementEvidence::issue(&inventory, &route, &mut authority)
                .unwrap_err()
                .code(),
            "migration-replacement-evidence-issuance-refused"
        );
    }
}

#[test]
fn replacement_evidence_is_one_shot_and_authority_is_revalidated_after_consumption() {
    let inventory = clean_inventory();
    let first_route = route("LEGACY-SKILL:old", "SKILL:current");
    let mut authority = TestReplacementAuthority::current(&inventory, &first_route);
    let first = evidence(&inventory, &first_route, &mut authority);
    let replay = evidence(&inventory, &first_route, &mut authority);
    MigrationPlan::build(
        &inventory,
        vec![first_route.clone()],
        vec![first],
        &mut authority,
    )
    .unwrap();
    assert_eq!(
        MigrationPlan::build(&inventory, vec![first_route], vec![replay], &mut authority,)
            .unwrap_err()
            .code(),
        "migration-replacement-evidence-replayed"
    );

    let route = route("LEGACY-SKILL:old", "SKILL:current");
    let mut mutating = TestReplacementAuthority::current(&inventory, &route);
    let evidence = evidence(&inventory, &route, &mut mutating);
    mutating.mutate_after_consume = true;
    assert_eq!(
        MigrationPlan::build(&inventory, vec![route], vec![evidence], &mut mutating)
            .unwrap_err()
            .code(),
        "migration-replacement-evidence-consumption-refused"
    );
}

#[test]
fn final_reconciliation_rejects_substituted_consumption_commitment() {
    let inventory = clean_inventory();
    let (mut substituted_plan, mut replacement_authority) = plan_with_authority(&inventory);
    let (review, mut review_authority) = issue_review(
        &substituted_plan,
        &inventory,
        Od009Decision::PreservePhysicalArtifact,
        "retirement-reviewer",
    );
    substituted_plan.substitute_replacement_consumption_for_test(sha('0'));
    let decision = RetirementDecision::reconcile_preservation(
        &substituted_plan,
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
            .contains(&"migration-replacement-evidence-invalid".to_owned())
    );

    let (plan, mut replacement_authority) = plan_with_authority(&inventory);
    let (review, mut review_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::PreservePhysicalArtifact,
        "retirement-reviewer",
    );
    replacement_authority.now = 2_001;
    let expired = RetirementDecision::reconcile_preservation(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &mut replacement_authority,
        &mut review_authority,
    );
    assert!(
        expired
            .reasons
            .contains(&"migration-replacement-ledger-not-current".to_owned())
    );
}

#[test]
fn final_reconciliation_requires_the_originating_persistent_replacement_ledger() {
    let inventory = clean_inventory();

    let (plan, original) = plan_with_authority(&inventory);
    let mut dropped = original.detached_without_ledger();
    let decision = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut dropped,
        "retirement-reviewer",
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"migration-replacement-ledger-not-current".to_owned())
    );

    let (plan, mut rolled_back) = plan_with_authority(&inventory);
    rolled_back.rollback_consumption();
    let decision = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut rolled_back,
        "retirement-reviewer",
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"migration-replacement-ledger-not-current".to_owned())
    );

    let (plan, mut revoked) = plan_with_authority(&inventory);
    revoked.revoke();
    let decision = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut revoked,
        "retirement-reviewer",
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"migration-replacement-ledger-not-current".to_owned())
    );

    let (plan, original) = plan_with_authority(&inventory);
    let mut substituted = original.clone();
    substituted.session_id = sha('7');
    let decision = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut substituted,
        "retirement-reviewer",
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"migration-replacement-ledger-not-current".to_owned())
    );
}
