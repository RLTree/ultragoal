#[test]
fn final_session_rotation_invalidates_mutate_restore_reuse() {
    let inventory = clean_inventory();
    let (plan, mut replacement_authority) = plan_with_authority(&inventory);
    let (review, mut review_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::PreservePhysicalArtifact,
        "retirement-reviewer",
    );
    let mut restored_bytes_new_session = inventory.clone();
    restored_bytes_new_session.rotate_session_for_test(sha('9'));
    assert_eq!(
        plan.verify_current(&restored_bytes_new_session)
            .unwrap_err()
            .code(),
        "migration-plan-stale"
    );
    let decision = RetirementDecision::reconcile_preservation(
        &plan,
        "retire-LEGACY-SKILL:old",
        &restored_bytes_new_session,
        &review,
        &mut replacement_authority,
        &mut review_authority,
    );
    assert!(
        decision
            .reasons
            .contains(&"migration-replacement-evidence-invalid".to_owned())
    );
}

#[test]
fn clean_target_is_only_a_non_authoritative_preservation_candidate() {
    let inventory = clean_inventory();
    let (plan, mut replacement_authority) = plan_with_authority(&inventory);
    let decision = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut replacement_authority,
        "retirement-reviewer",
    );
    assert_eq!(
        decision.status,
        RetirementStatus::NonAuthoritativePreservationCandidate
    );
    assert_eq!(
        decision.claim_ceiling,
        "migration_candidate_not_adoption_or_retirement"
    );
}

#[test]
fn active_readers_writers_routes_and_generated_authority_block_retirement() {
    let inventory = inventory_with_source(
        surface(
            "LEGACY-SKILL:old",
            SurfaceStatus::Active,
            vec!["reader-a".to_owned()],
            vec!["writer-a".to_owned()],
            vec!["legacy-command".to_owned()],
            vec!["docs/generated/old.json".to_owned()],
        ),
        'f',
    );
    let (plan, mut replacement_authority) = plan_with_authority(&inventory);
    let decision = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut replacement_authority,
        "retirement-reviewer",
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    for expected in [
        "migration-active-readers-remain",
        "migration-active-writers-remain",
        "migration-public-routes-remain",
        "migration-generated-authority-remains",
    ] {
        assert!(decision.reasons.contains(&expected.to_owned()));
    }
}

#[test]
fn compatibility_usage_or_open_window_blocks_retirement() {
    let inventory = clean_inventory();
    let route = CompatibilityRoute::new(CompatibilityRouteDefinition {
        route_id: "route-old-to-current".to_owned(),
        source_id: "LEGACY-SKILL:old".to_owned(),
        canonical_target_id: "SKILL:current".to_owned(),
        owner_id: "migration-owner".to_owned(),
        warning: "Compatibility warning remains visible".to_owned(),
        usage_measurement_sha256: sha('1'),
        compatibility_boundary: "version-2-boundary".to_owned(),
        removal_condition: "zero-active-references".to_owned(),
        equivalence_sha256: sha('2'),
        observed_invocations: 3,
        compatibility_window_complete: false,
    });
    let mut authority = TestReplacementAuthority::current(&inventory, &route);
    let evidence = evidence(&inventory, &route, &mut authority);
    let plan =
        MigrationPlan::build(&inventory, vec![route], vec![evidence], &mut authority).unwrap();
    let decision = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut authority,
        "retirement-reviewer",
    );
    assert!(
        decision
            .reasons
            .contains(&"migration-compatibility-window-open".to_owned())
    );
}

#[test]
fn route_owner_and_replacement_reviewer_cannot_self_accept_retirement() {
    let inventory = clean_inventory();
    let (plan, _replacement_authority) = plan_with_authority(&inventory);
    for reviewer in ["migration-owner", "replacement-reviewer"] {
        let mut authority =
            TestReviewAuthority::current(&inventory, Od009Decision::PreservePhysicalArtifact);
        authority.reviewer_id = reviewer.to_owned();
        let error =
            RetirementReview::issue(&plan, "retire-LEGACY-SKILL:old", &inventory, &mut authority)
                .unwrap_err();
        assert_eq!(error.code(), "migration-retirement-review-issuance-refused");
    }
}

#[test]
fn destructive_retirement_requires_separate_authority() {
    let inventory = clean_inventory();
    let (plan, mut replacement_authority) = plan_with_authority(&inventory);
    let (review, mut review_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::RequestPhysicalDeletion,
        "retirement-reviewer",
    );
    let blocked = RetirementDecision::reconcile_preservation(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &mut replacement_authority,
        &mut review_authority,
    );
    assert_eq!(blocked.status, RetirementStatus::Blocked);
    assert!(
        blocked
            .reasons
            .contains(&"migration-destructive-authority-required".to_owned())
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
    let authorized = RetirementDecision::reconcile_destructive(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &authorization,
        &mut replacement_authority,
        &mut review_authority,
        &mut effect_authority,
    );
    assert_eq!(
        authorized.status,
        RetirementStatus::DestructiveRetirementCandidate
    );
}
