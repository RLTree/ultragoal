#[test]
fn stale_registry_candidate_session_plan_projection_and_authority_rebinding_are_rejected() {
    let input = retirement_input();
    let plan = derive_plan(&input).unwrap();

    let mut stale_registry_route = route(
        "route-old-to-current",
        "LEGACY-SKILL:old",
        "skills/old/SKILL.md",
        "SKILL:current",
        'a',
        'b',
        Some("retirement"),
    );
    stale_registry_route["transition"]["adopted_effect"]["behavior_execution_sha256"] =
        json!(sha('f'));
    let stale_input = input_with_routes(
        input.inventory().surfaces().to_vec(),
        vec![stale_registry_route],
        '0',
    );
    let store = FakeStore::default();
    let mut stale_source = FakeSource::new(stale_input);
    let mut authority = FakeAuthority::current(&plan, 'a');
    assert_eq!(
        issue_apply_authorization(&plan, &mut stale_source, &mut authority, &store)
            .unwrap_err()
            .code(),
        "migration-product-plan-stale-or-substituted"
    );

    let rebound_input = input_with_routes(
        input.inventory().surfaces().to_vec(),
        vec![route(
            "route-old-to-current",
            "LEGACY-SKILL:old",
            "skills/old/SKILL.md",
            "SKILL:current",
            'a',
            'b',
            Some("retirement"),
        )],
        '1',
    );
    let mut rebound_source = FakeSource::new(rebound_input);
    assert_eq!(
        issue_apply_authorization(&plan, &mut rebound_source, &mut authority, &store)
            .unwrap_err()
            .code(),
        "migration-product-plan-stale-or-substituted"
    );

    let mut projection_value = serde_json::to_value(plan.projection()).unwrap();
    projection_value["plan_sha256"] = json!(sha('0'));
    let substituted: ProductMigrationPlanProjection =
        serde_json::from_value(projection_value).unwrap();
    assert_eq!(
        plan.verify_projection(&substituted).unwrap_err().code(),
        "migration-product-plan-projection-substituted"
    );

    let store = FakeStore::default();
    let mut current_source = FakeSource::new(input);
    let mut issuer = FakeAuthority::current(&plan, 'b');
    let token = issue_apply_authorization(&plan, &mut current_source, &mut issuer, &store).unwrap();
    issuer.input_binding = sha('0');
    let mut effects = FakeEffects::for_plan(&plan);
    assert_eq!(
        apply_product_plan(
            &plan,
            &token,
            &mut current_source,
            &issuer,
            &store,
            &mut effects,
        )
        .unwrap_err()
        .code(),
        "migration-product-authorization-stale-or-rebound"
    );
}

#[test]
fn authorization_expiry_source_race_and_concurrent_semantic_reservation_fail_closed() {
    let input = retirement_input();
    let plan = derive_plan(&input).unwrap();
    let store = Arc::new(FakeStore::default());

    let mut expired_source = FakeSource::new(input.clone());
    let mut expired = FakeAuthority::current(&plan, 'a');
    expired.now = expired.expires + 1;
    assert_eq!(
        issue_apply_authorization(&plan, &mut expired_source, &mut expired, &*store)
            .unwrap_err()
            .code(),
        "migration-product-authorization-issuer-refused"
    );

    let mut source = FakeSource::new(input.clone());
    let mut authority = FakeAuthority::current(&plan, 'b');
    let token = issue_apply_authorization(&plan, &mut source, &mut authority, &*store).unwrap();
    source.stale = true;
    let mut effects = FakeEffects::for_plan(&plan);
    assert_eq!(
        apply_product_plan(
            &plan,
            &token,
            &mut source,
            &authority,
            &*store,
            &mut effects,
        )
        .unwrap_err()
        .code(),
        "test-migration-source-stale"
    );

    let store = Arc::new(FakeStore::default());
    let shared_effects = FakeEffects::for_plan(&plan);
    let mut source_a = FakeSource::new(input.clone());
    let mut source_b = FakeSource::new(input);
    let mut authority_a = FakeAuthority::current(&plan, 'c');
    let mut authority_b = FakeAuthority::current(&plan, 'd');
    let token_a =
        issue_apply_authorization(&plan, &mut source_a, &mut authority_a, &*store).unwrap();
    let token_b =
        issue_apply_authorization(&plan, &mut source_b, &mut authority_b, &*store).unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for (mut source, authority, token) in [
        (source_a, authority_a, token_a),
        (source_b, authority_b, token_b),
    ] {
        let store = store.clone();
        let plan = plan.clone();
        let barrier = barrier.clone();
        let mut effects = shared_effects.clone();
        handles.push(thread::spawn(move || {
            barrier.wait();
            apply_product_plan(
                &plan,
                &token,
                &mut source,
                &authority,
                &*store,
                &mut effects,
            )
        }));
    }
    let results = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
    assert_eq!(shared_effects.counts().0, 1);
}
