#[test]
fn compatibility_recovery_before_effect_refuses_after_deadline_crossing_with_zero_effect() {
    let input = compatibility_input();
    let plan = derive_plan(&input).unwrap();
    let store = FakeStore::default();
    store.fail_on_cas(1);
    let mut source = FakeSource::new(input);
    let mut authority = FakeAuthority::current(&plan, 'a');
    let token = issue_apply_authorization(&plan, &mut source, &mut authority, &store).unwrap();
    let mut effects = FakeEffects::for_plan(&plan);
    assert_eq!(
        apply_product_plan(&plan, &token, &mut source, &authority, &store, &mut effects,)
            .unwrap_err()
            .code(),
        "migration-product-operation-interrupted"
    );
    assert_eq!(effects.counts(), (0, 0));
    authority.boundary_sequence += 100;
    authority.boundary_now = 86_402_000;
    let operation = store.only_operation();
    assert_eq!(
        recover_product_operation(
            operation.operation_id(),
            &plan,
            &mut source,
            &authority,
            &store,
            &mut effects,
        )
        .unwrap_err()
        .code(),
        "migration-product-compatibility-boundary-crossed"
    );
    assert_eq!(effects.counts(), (0, 0));
    assert_eq!(
        effects.authority(plan.effects()[0].effect_id()),
        plan.effects()[0].before().clone()
    );
}

#[test]
fn compatibility_recovery_reconciles_pre_boundary_effect_without_duplicate_after_crossing() {
    let input = compatibility_input();
    let plan = derive_plan(&input).unwrap();
    let store = FakeStore::default();
    store.fail_on_cas(3);
    let mut source = FakeSource::new(input);
    let mut authority = FakeAuthority::current(&plan, 'b');
    let token = issue_apply_authorization(&plan, &mut source, &mut authority, &store).unwrap();
    let mut effects = FakeEffects::for_plan(&plan);
    assert_eq!(
        apply_product_plan(&plan, &token, &mut source, &authority, &store, &mut effects,)
            .unwrap_err()
            .code(),
        "migration-product-operation-interrupted"
    );
    assert_eq!(effects.counts(), (1, 0));
    authority.boundary_sequence += 100;
    authority.boundary_now = 86_402_000;
    let operation = store.only_operation();
    let outcome = recover_product_operation(
        operation.operation_id(),
        &plan,
        &mut source,
        &authority,
        &store,
        &mut effects,
    )
    .unwrap();
    assert_eq!(outcome.status(), ApplyOutcomeStatus::AlreadyApplied);
    assert_eq!(effects.counts(), (1, 0));

    let replay = recover_product_operation(
        operation.operation_id(),
        &plan,
        &mut source,
        &authority,
        &store,
        &mut effects,
    )
    .unwrap();
    assert_eq!(replay.status(), ApplyOutcomeStatus::AlreadyApplied);
    assert_eq!(effects.counts(), (1, 0));
}

#[test]
fn compatibility_recovery_with_unbound_completion_timing_is_ambiguous_and_never_duplicates() {
    let input = compatibility_input();
    let plan = derive_plan(&input).unwrap();
    let store = FakeStore::default();
    store.fail_on_cas(3);
    let mut source = FakeSource::new(input);
    let mut authority = FakeAuthority::current(&plan, 'c');
    let token = issue_apply_authorization(&plan, &mut source, &mut authority, &store).unwrap();
    let mut effects = FakeEffects::for_plan(&plan);
    assert!(
        apply_product_plan(&plan, &token, &mut source, &authority, &store, &mut effects,).is_err()
    );
    assert_eq!(effects.counts(), (1, 0));
    effects.substitute_effect_permit(plan.effects()[0].effect_id(), Some(sha('f')));
    authority.boundary_sequence += 100;
    authority.boundary_now = 86_402_000;
    let operation = store.only_operation();
    let outcome = recover_product_operation(
        operation.operation_id(),
        &plan,
        &mut source,
        &authority,
        &store,
        &mut effects,
    )
    .unwrap();
    assert_eq!(outcome.status(), ApplyOutcomeStatus::Ambiguous);
    assert_eq!(effects.counts(), (1, 0));
    assert!(outcome.terminal_proof_sha256().is_none());
}
