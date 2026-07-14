#[test]
fn authorization_rejects_crossed_boundary_source_substitution_and_version_rollback() {
    let input = compatibility_input();
    let plan = derive_plan(&input).unwrap();

    let store = FakeStore::default();
    let mut source = FakeSource::new(input.clone());
    let mut crossed = FakeAuthority::current(&plan, 'a');
    crossed.boundary_now = 86_402_000;
    assert_eq!(
        issue_apply_authorization(&plan, &mut source, &mut crossed, &store)
            .unwrap_err()
            .code(),
        "migration-product-compatibility-boundary-crossed"
    );
    assert_eq!(store.operation_count(), 0);

    let mut source = FakeSource::new(input.clone());
    let mut substituted = FakeAuthority::current(&plan, 'b');
    substituted.boundary_source_identity = sha('e');
    assert_eq!(
        issue_apply_authorization(&plan, &mut source, &mut substituted, &store)
            .unwrap_err()
            .code(),
        "migration-product-compatibility-boundary-observation-stale-or-substituted"
    );

    let mut source = FakeSource::new(input);
    let mut rolled_back = FakeAuthority::current(&plan, 'c');
    rolled_back.current_product_version = "0.0.11".to_owned();
    assert_eq!(
        issue_apply_authorization(&plan, &mut source, &mut rolled_back, &store)
            .unwrap_err()
            .code(),
        "migration-product-compatibility-boundary-observation-stale-or-substituted"
    );
}

#[test]
fn crossing_after_authorization_but_immediately_before_effect_has_zero_effect_or_target_state_change()
 {
    let input = compatibility_input();
    let plan = derive_plan(&input).unwrap();
    let store = FakeStore::default();
    let mut source = FakeSource::new(input);
    let mut authority = FakeAuthority::current(&plan, 'd');
    authority.cross_deadline_after_captures(3, 86_402_000);
    let token = issue_apply_authorization(&plan, &mut source, &mut authority, &store).unwrap();
    let mut effects = FakeEffects::for_plan(&plan);
    assert_eq!(
        apply_product_plan(&plan, &token, &mut source, &authority, &store, &mut effects,)
            .unwrap_err()
            .code(),
        "migration-product-compatibility-boundary-crossed"
    );
    assert_eq!(effects.counts(), (0, 0));
    assert_eq!(
        effects.authority(plan.effects()[0].effect_id()),
        plan.effects()[0].before().clone()
    );
    assert!(effects.compatibility_prerequisite_inputs().is_empty());

    let mut route = compatibility_route();
    let boundary = route["transition"]["adopted_effect"]["compatibility_prerequisites"]["boundary"]
        .as_object_mut()
        .unwrap();
    boundary.remove("deadline_unix_ms");
    boundary.insert("product_version".to_owned(), json!("0.0.13"));
    let input = compatibility_input_with_route(route, '0');
    let plan = derive_plan(&input).unwrap();
    let store = FakeStore::default();
    let mut source = FakeSource::new(input);
    let mut authority = FakeAuthority::current(&plan, 'd');
    authority.cross_version_after_captures(3, "0.0.13");
    let token = issue_apply_authorization(&plan, &mut source, &mut authority, &store).unwrap();
    let mut effects = FakeEffects::for_plan(&plan);
    assert_eq!(
        apply_product_plan(&plan, &token, &mut source, &authority, &store, &mut effects,)
            .unwrap_err()
            .code(),
        "migration-product-compatibility-boundary-crossed"
    );
    assert_eq!(effects.counts(), (0, 0));
}

#[test]
fn boundary_source_or_current_version_substitution_after_authorization_refuses_before_reservation()
{
    for substitution in ["source", "version", "seal-key"] {
        let input = compatibility_input();
        let plan = derive_plan(&input).unwrap();
        let store = FakeStore::default();
        let mut source = FakeSource::new(input);
        let mut authority = FakeAuthority::current(&plan, 'e');
        let token = issue_apply_authorization(&plan, &mut source, &mut authority, &store).unwrap();
        match substitution {
            "source" => authority.boundary_source_identity = sha('f'),
            "version" => authority.current_product_version = "0.0.11".to_owned(),
            "seal-key" => authority.boundary_key = "rotated-boundary-seal-key".to_owned(),
            _ => unreachable!(),
        }
        let mut effects = FakeEffects::for_plan(&plan);
        let expected = if substitution == "version" {
            "migration-product-compatibility-boundary-observation-stale-or-substituted"
        } else {
            "migration-product-authorization-stale-or-rebound"
        };
        assert_eq!(
            apply_product_plan(&plan, &token, &mut source, &authority, &store, &mut effects,)
                .unwrap_err()
                .code(),
            expected
        );
        assert_eq!(store.operation_count(), 0);
        assert_eq!(effects.counts(), (0, 0));
    }
}

#[test]
fn compatibility_boundary_journal_projection_and_permit_mutation_fail_closed() {
    let input = compatibility_input();
    let plan = derive_plan(&input).unwrap();

    let mut projection_value = serde_json::to_value(plan.projection()).unwrap();
    projection_value["compatibility_boundary_binding"]["initial_observation"]["current_product_version"] =
        json!("0.0.11");
    let substituted: ProductMigrationPlanProjection =
        serde_json::from_value(projection_value).unwrap();
    assert_eq!(
        plan.verify_projection(&substituted).unwrap_err().code(),
        "migration-product-plan-projection-substituted"
    );

    for (crash_cas, mutation_path) in [(1, "last-observation"), (3, "pending-permit-observation")] {
        let store = FakeStore::default();
        store.fail_on_cas(crash_cas);
        let mut source = FakeSource::new(input.clone());
        let mut authority = FakeAuthority::current(&plan, if crash_cas == 1 { '8' } else { '9' });
        let token = issue_apply_authorization(&plan, &mut source, &mut authority, &store).unwrap();
        let mut effects = FakeEffects::for_plan(&plan);
        assert_eq!(
            apply_product_plan(&plan, &token, &mut source, &authority, &store, &mut effects,)
                .unwrap_err()
                .code(),
            "migration-product-operation-interrupted"
        );
        let interrupted = store.only_operation();
        let operation_id = interrupted.operation_id().to_owned();
        let mut value = serde_json::to_value(interrupted).unwrap();
        match mutation_path {
            "last-observation" => {
                value["last_boundary_observation"]["observed_at_unix_ms"] = json!(10_001)
            }
            "pending-permit-observation" => {
                value["pending_effect_permit"]["boundary_observation"]["current_product_version"] =
                    json!("0.0.11")
            }
            _ => unreachable!(),
        }
        store.substitute_only_operation_from_json(value);
        assert_eq!(
            recover_product_operation(
                &operation_id,
                &plan,
                &mut source,
                &authority,
                &store,
                &mut effects,
            )
            .unwrap_err()
            .code(),
            "migration-product-recovery-substituted"
        );
    }
}
