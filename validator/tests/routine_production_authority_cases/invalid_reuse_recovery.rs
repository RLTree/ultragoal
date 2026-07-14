use super::*;

#[test]
pub(crate) fn invalid_reuse_never_regresses_complete_or_blocks_later_exact_reuse() {
    let fixture = fixture("production-invalid-reuse-preserves-complete", true);
    let authority = AuthorityRoot::new("production-invalid-reuse-preserves-complete");
    let first = mediate(&authority, &fixture, prepared(&fixture), Vec::new()).unwrap();
    let valid_reuse = first.reuse_artifacts().to_vec();

    let before_malformed = authority.tree();
    let malformed = mediate(
        &authority,
        &fixture,
        prepared(&fixture),
        vec![b"not-a-reuse-artifact".to_vec()],
    )
    .unwrap_err();
    assert_eq!(
        malformed.cause(),
        "mediator-production-reuse-input-malformed"
    );
    assert_eq!(authority.tree(), before_malformed);

    let exact_after_malformed = mediate(
        &authority,
        &fixture,
        prepared(&fixture),
        valid_reuse.clone(),
    )
    .unwrap();
    assert_eq!(
        exact_after_malformed.nodes()[0].disposition(),
        RoutineNodeDisposition::Reused
    );

    let forged = corrupt_reuse_witness(&valid_reuse[0]);
    let before_forgery_state = authority_state(&authority);
    let before_forgery_cardinalities = authority_cardinalities(&authority);
    let before_forgery_tree = fixture.repo.tree();
    let before_forgery_status = fixture.repo.status();
    let refused = mediate(&authority, &fixture, prepared(&fixture), vec![forged]).unwrap_err();
    assert_eq!(
        refused.cause(),
        "mediator-production-reuse-not-authenticated"
    );
    assert_eq!(authority_state(&authority), before_forgery_state);
    assert_eq!(
        authority_cardinalities(&authority),
        before_forgery_cardinalities
    );
    assert_eq!(fixture.repo.tree(), before_forgery_tree);
    assert_eq!(fixture.repo.status(), before_forgery_status);

    // A foreign artifact is fully canonical, internally self-consistent, and
    // process-authenticated, but it belongs to a different durable Complete
    // record. Repeating it must not consume grants or create an exhaustion
    // path in this authority store.
    let foreign_reuse = foreign_reuse_for_same_request(&fixture, "production-foreign-complete");
    let local_wire: serde_json::Value = serde_json::from_slice(&valid_reuse[0]).unwrap();
    let foreign_wire: serde_json::Value = serde_json::from_slice(&foreign_reuse[0]).unwrap();
    assert_eq!(local_wire["protocol_id"], foreign_wire["protocol_id"]);
    assert_eq!(local_wire["intent_id"], foreign_wire["intent_id"]);
    assert_ne!(valid_reuse[0], foreign_reuse[0]);
    let before_substitution_state = authority_state(&authority);
    let before_substitution_cardinalities = authority_cardinalities(&authority);
    let before_substitution_tree = fixture.repo.tree();
    let before_substitution_status = fixture.repo.status();
    for _ in 0..64 {
        let error = mediate(
            &authority,
            &fixture,
            prepared(&fixture),
            foreign_reuse.clone(),
        )
        .unwrap_err();
        assert_eq!(error.cause(), "mediator-production-reuse-not-authenticated");
    }
    assert_eq!(authority_state(&authority), before_substitution_state);
    assert_eq!(
        authority_cardinalities(&authority),
        before_substitution_cardinalities
    );
    assert_eq!(fixture.repo.tree(), before_substitution_tree);
    assert_eq!(fixture.repo.status(), before_substitution_status);

    ProductionRoutineIssuer::open(authority.path()).unwrap();

    let exact_after_forgery =
        mediate(&authority, &fixture, prepared(&fixture), valid_reuse).unwrap();
    assert_eq!(
        exact_after_forgery.nodes()[0].disposition(),
        RoutineNodeDisposition::Reused
    );
}

#[test]
pub(crate) fn reuse_preauthorization_generation_race_fails_before_reservation_mutation() {
    let fixture = fixture("production-reuse-preauthorization-race", true);
    let authority = AuthorityRoot::new("production-reuse-preauthorization-race");
    let first = mediate(&authority, &fixture, prepared(&fixture), Vec::new()).unwrap();
    let reuse = first.reuse_artifacts().to_vec();
    assert_eq!(authority_cardinalities(&authority), (1, 1, 1));

    let raced_state = Arc::new(Mutex::new(None));
    let raced_state_from_hook = Arc::clone(&raced_state);
    let authority_root = authority.path().to_path_buf();
    ProductionRoutineIssuer::test_set_reuse_preauthorization_hook(move || {
        let contender = ProductionRoutineIssuer::open(&authority_root).unwrap();
        contender.test_seed_capacity(1, 1).unwrap();
        *raced_state_from_hook.lock().unwrap() =
            Some(fs::read(authority_root.join("routine-authority.state")).unwrap());
    });
    let before_target = fixture.repo.tree();
    let before_status = fixture.repo.status();
    let refused = mediate(&authority, &fixture, prepared(&fixture), reuse.clone()).unwrap_err();
    assert_eq!(
        refused.cause(),
        "routine-production-reuse-preauthorization-stale"
    );
    let after_contender = raced_state.lock().unwrap().take().unwrap();
    assert_eq!(authority_state(&authority), after_contender);
    assert_eq!(authority_cardinalities(&authority), (1, 1, 1));
    assert_eq!(fixture.repo.tree(), before_target);
    assert_eq!(fixture.repo.status(), before_status);

    let reopened = mediate(&authority, &fixture, prepared(&fixture), reuse).unwrap();
    assert_eq!(
        reopened.nodes()[0].disposition(),
        RoutineNodeDisposition::Reused
    );
}

#[test]
pub(crate) fn consumed_grant_max_minus_one_max_and_max_plus_one_are_fail_closed() {
    let (record_limit, consumed_limit) = ProductionRoutineIssuer::test_capacity_limits();
    assert_eq!((record_limit, consumed_limit), (4_096, 16_384));
    let fixture = fixture("production-consumed-capacity", true);
    let authority = AuthorityRoot::new("production-consumed-capacity");
    let first = mediate(&authority, &fixture, prepared(&fixture), Vec::new()).unwrap();
    let reuse = first.reuse_artifacts().to_vec();
    let foreign_reuse =
        foreign_reuse_for_same_request(&fixture, "production-consumed-capacity-foreign");
    let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
    issuer.test_seed_capacity(1, consumed_limit - 1).unwrap();
    assert_eq!(
        authority_cardinalities(&authority),
        (1, 1, consumed_limit - 1)
    );

    let at_max = mediate(&authority, &fixture, prepared(&fixture), reuse.clone()).unwrap();
    assert_eq!(
        at_max.nodes()[0].disposition(),
        RoutineNodeDisposition::Reused
    );
    assert_eq!(authority_cardinalities(&authority), (1, 1, consumed_limit));
    let max_state = authority_state(&authority);

    let forged_at_max =
        mediate(&authority, &fixture, prepared(&fixture), foreign_reuse).unwrap_err();
    assert_eq!(
        forged_at_max.cause(),
        "mediator-production-reuse-not-authenticated"
    );
    assert_eq!(authority_state(&authority), max_state);

    let over_limit = mediate(&authority, &fixture, prepared(&fixture), reuse).unwrap_err();
    assert_eq!(
        over_limit.cause(),
        "routine-production-authority-capacity-exhausted"
    );
    assert_eq!(authority_state(&authority), max_state);

    let reopened = ProductionRoutineIssuer::open(authority.path()).unwrap();
    assert!(
        reopened
            .pending_recovery(&fixture.context, &fixture.plan, &prepared(&fixture))
            .unwrap()
            .is_none()
    );
    let replay = mediate(&authority, &fixture, prepared(&fixture), Vec::new()).unwrap_err();
    assert_eq!(
        replay.cause(),
        "routine-production-semantic-effect-replayed"
    );
    assert_eq!(authority_state(&authority), max_state);
}
