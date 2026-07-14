use super::*;

#[test]
pub(crate) fn protocol_effect_max_minus_one_max_and_max_plus_one_are_fail_closed() {
    let (record_limit, consumed_limit) = ProductionRoutineIssuer::test_capacity_limits();
    let fixture_at_max = fixture("production-record-capacity-max", true);
    let fixture_over_limit = fixture("production-record-capacity-over", true);
    let authority = AuthorityRoot::new("production-record-capacity");
    let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
    issuer
        .test_seed_capacity(record_limit - 1, record_limit - 1)
        .unwrap();
    assert_eq!(
        authority_cardinalities(&authority),
        (record_limit - 1, record_limit - 1, record_limit - 1)
    );

    let request_at_max = prepared(&fixture_at_max);
    issuer
        .test_reserve_and_abandon(
            &fixture_at_max.context,
            &fixture_at_max.plan,
            &request_at_max,
            false,
        )
        .unwrap();
    assert_eq!(
        authority_cardinalities(&authority),
        (record_limit, record_limit, record_limit)
    );

    let reopened = ProductionRoutineIssuer::open(authority.path()).unwrap();
    let recovery_request = prepared(&fixture_at_max);
    let recovery = reopened
        .pending_recovery(
            &fixture_at_max.context,
            &fixture_at_max.plan,
            &recovery_request,
        )
        .unwrap()
        .expect("the exact maximum record remains recoverable after reopen");
    let recovered = reopened
        .mediate(
            &fixture_at_max.context,
            &fixture_at_max.plan,
            recovery_request,
            Some(recovery),
            RoutineCancellation::new(),
            RoutineReuseInput::new(Vec::new()),
        )
        .unwrap();
    assert_eq!(recovered.status(), RoutineMediatorStatus::CompleteExecution);
    assert_eq!(
        authority_cardinalities(&authority),
        (record_limit, record_limit, record_limit + 1)
    );
    assert!(record_limit + 1 < consumed_limit);
    let max_state = authority_state(&authority);

    let over_limit_request = prepared(&fixture_over_limit);
    let refused = reopened
        .test_reserve_and_abandon(
            &fixture_over_limit.context,
            &fixture_over_limit.plan,
            &over_limit_request,
            false,
        )
        .unwrap_err();
    assert_eq!(
        refused.cause(),
        "routine-production-authority-capacity-exhausted"
    );
    assert_eq!(authority_state(&authority), max_state);
    assert_eq!(
        authority_cardinalities(&authority),
        (record_limit, record_limit, record_limit + 1)
    );
    ProductionRoutineIssuer::open(authority.path()).unwrap();

    let replay = mediate(
        &authority,
        &fixture_at_max,
        prepared(&fixture_at_max),
        Vec::new(),
    )
    .unwrap_err();
    assert_eq!(
        replay.cause(),
        "routine-production-semantic-effect-replayed"
    );
    assert_eq!(authority_state(&authority), max_state);
}

#[test]
pub(crate) fn no_op_bypasses_authority_initialization_and_all_writes() {
    let fixture = fixture("production-noop", false);
    assert_eq!(fixture.plan.affected_set().mode(), PlanMode::NoOp);
    let authority = AuthorityRoot::new("production-noop");
    fs::remove_dir(authority.path()).unwrap();
    let before = fixture.repo.tree();
    let prepared = prepare_routine_execution(
        &fixture.context,
        &fixture.graph,
        &fixture.snapshot,
        &fixture.plan,
        RoutineAdapterSpec::new("routine", Vec::new()),
    )
    .unwrap();
    let result = mediate_prepared_routine_execution_production(
        authority.path(),
        &fixture.context,
        &fixture.plan,
        prepared,
        None,
        RoutineCancellation::new(),
        RoutineReuseInput::new(Vec::new()),
    )
    .unwrap();
    assert_eq!(result.status(), RoutineMediatorStatus::CompleteNoOp);
    assert!(!authority.path().exists());
    assert_eq!(fixture.repo.tree(), before);
}

#[test]
pub(crate) fn reserved_and_started_crashes_require_exact_bounded_recovery() {
    for started in [false, true] {
        let fixture = fixture(
            if started {
                "production-crash-started"
            } else {
                "production-crash-reserved"
            },
            true,
        );
        let authority = AuthorityRoot::new(if started {
            "production-crash-started"
        } else {
            "production-crash-reserved"
        });
        let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
        let request = prepared(&fixture);
        issuer
            .test_reserve_and_abandon(&fixture.context, &fixture.plan, &request, started)
            .unwrap();
        let before_target = fixture.repo.tree();
        let before_authority = authority.tree();
        let replay = issuer
            .mediate(
                &fixture.context,
                &fixture.plan,
                request,
                None,
                RoutineCancellation::new(),
                RoutineReuseInput::new(Vec::new()),
            )
            .unwrap_err();
        assert_eq!(
            replay.cause(),
            "routine-production-semantic-effect-replayed"
        );
        assert_eq!(fixture.repo.tree(), before_target);
        assert_eq!(authority.tree(), before_authority);

        let recovery_request = prepared(&fixture);
        let recovery = issuer
            .pending_recovery(&fixture.context, &fixture.plan, &recovery_request)
            .unwrap()
            .expect("pending recovery authority");
        let recovered = issuer
            .mediate(
                &fixture.context,
                &fixture.plan,
                recovery_request,
                Some(recovery),
                RoutineCancellation::new(),
                RoutineReuseInput::new(Vec::new()),
            )
            .unwrap();
        assert_eq!(recovered.status(), RoutineMediatorStatus::CompleteExecution);
        assert!(
            issuer
                .pending_recovery(&fixture.context, &fixture.plan, &prepared(&fixture))
                .unwrap()
                .is_none()
        );
    }
}

#[test]
pub(crate) fn expired_recovery_authority_is_rejected_from_authenticated_state() {
    let fixture = fixture("production-expired-recovery", true);
    let authority = AuthorityRoot::new("production-expired-recovery");
    let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
    let request = prepared(&fixture);
    issuer
        .test_reserve_and_abandon(&fixture.context, &fixture.plan, &request, true)
        .unwrap();
    issuer
        .test_expire_pending(&fixture.context, &fixture.plan, &request)
        .unwrap();
    let before_target = fixture.repo.tree();
    let before_authority = authority.tree();
    let refused = issuer
        .pending_recovery(&fixture.context, &fixture.plan, &request)
        .unwrap_err();
    assert_eq!(refused.cause(), "routine-production-recovery-expired");
    assert_eq!(fixture.repo.tree(), before_target);
    assert_eq!(authority.tree(), before_authority);
}

#[test]
pub(crate) fn preterminal_reconciliation_failure_remains_pending_across_reopen() {
    let fixture = fixture("production-preterminal-crash", true);
    let authority = AuthorityRoot::new("production-preterminal-crash");
    set_test_mediator_finish_failure();
    let failed = mediate(&authority, &fixture, prepared(&fixture), Vec::new()).unwrap_err();
    assert_eq!(failed.cause(), "adapter-mediation-transition-incomplete");

    let reopened = ProductionRoutineIssuer::open(authority.path()).unwrap();
    let recovery_request = prepared(&fixture);
    let recovery = reopened
        .pending_recovery(&fixture.context, &fixture.plan, &recovery_request)
        .unwrap()
        .expect("started record remains pending");
    let result = reopened
        .mediate(
            &fixture.context,
            &fixture.plan,
            recovery_request,
            Some(recovery),
            RoutineCancellation::new(),
            RoutineReuseInput::new(Vec::new()),
        )
        .unwrap();
    assert_eq!(result.status(), RoutineMediatorStatus::CompleteExecution);
}
