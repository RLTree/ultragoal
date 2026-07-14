#[test]
fn persistent_replacement_ledger_rejects_cross_adapter_replay() {
    let inventory = clean_inventory();
    let (plan, mut original) = plan_with_authority(&inventory);
    let mut persistent_peer = original.clone();
    let first = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut original,
        "retirement-reviewer-one",
    );
    assert_eq!(
        first.status,
        RetirementStatus::NonAuthoritativePreservationCandidate
    );
    let replay = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut persistent_peer,
        "retirement-reviewer-two",
    );
    assert_eq!(replay.status, RetirementStatus::Blocked);
    assert!(
        replay
            .reasons
            .contains(&"migration-replacement-ledger-invalid-or-replayed".to_owned())
    );
}

#[test]
fn concurrent_final_reconciliation_elects_exactly_one_ledger_winner() {
    let inventory = clean_inventory();
    let (plan, authority) = plan_with_authority(&inventory);
    let plan = Arc::new(plan);
    let barrier = Arc::new(Barrier::new(3));
    let handles = ["concurrent-reviewer-one", "concurrent-reviewer-two"]
        .into_iter()
        .map(|reviewer| {
            let inventory = inventory.clone();
            let plan = Arc::clone(&plan);
            let mut authority = authority.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                preservation_decision(
                    plan.as_ref(),
                    "retire-LEGACY-SKILL:old",
                    &inventory,
                    &mut authority,
                    reviewer,
                )
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    let decisions = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        decisions
            .iter()
            .filter(|decision| {
                decision.status == RetirementStatus::NonAuthoritativePreservationCandidate
            })
            .count(),
        1
    );
    assert_eq!(
        decisions
            .iter()
            .filter(|decision| decision.status == RetirementStatus::Blocked)
            .count(),
        1
    );
    assert!(decisions.iter().any(|decision| {
        decision
            .reasons
            .contains(&"migration-replacement-ledger-invalid-or-replayed".to_owned())
    }));
}

#[test]
fn final_reconciliation_revalidates_replacement_ledger_around_every_consumption() {
    let inventory = clean_inventory();
    let (plan, mut rolled_back) = plan_with_authority(&inventory);
    rolled_back.rollback_after_final_claim = true;
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
            .contains(&"migration-replacement-ledger-invalid-or-replayed".to_owned())
    );

    let (plan, mut drift_after_review) = plan_with_authority(&inventory);
    drift_after_review.revoke_on_final_verify_call = Some(2);
    let decision = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut drift_after_review,
        "retirement-reviewer",
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"migration-replacement-ledger-drifted".to_owned())
    );

    let (plan, mut drift_after_effect) = plan_with_authority(&inventory);
    drift_after_effect.revoke_on_final_verify_call = Some(4);
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
    let decision = RetirementDecision::reconcile_destructive(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &authorization,
        &mut drift_after_effect,
        &mut review_authority,
        &mut effect_authority,
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"migration-replacement-ledger-drifted".to_owned())
    );
}
