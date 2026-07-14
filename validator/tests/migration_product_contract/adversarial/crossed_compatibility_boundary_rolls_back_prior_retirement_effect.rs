#[test]
fn crossed_compatibility_boundary_rolls_back_prior_retirement_effect() {
    let input = input_with_routes(
        vec![
            surface(
                "LEGACY-SKILL:retire-first",
                "legacy-skill",
                "skills/retire-first/SKILL.md",
                'a',
                SurfaceStatus::Candidate,
                &[],
                &[],
                &[],
                &[],
            ),
            surface(
                "LEGACY-SKILL:compat-second",
                "legacy-skill",
                "skills/compat-second/SKILL.md",
                'c',
                SurfaceStatus::Active,
                &["compat-reader"],
                &["compat-writer"],
                &["compat-public"],
                &["compat-generated"],
            ),
            surface(
                "SKILL:current",
                "skill",
                "skills/current/SKILL.md",
                'b',
                SurfaceStatus::Active,
                &[],
                &[],
                &[],
                &[],
            ),
        ],
        vec![
            route(
                "route-a-retire",
                "LEGACY-SKILL:retire-first",
                "skills/retire-first/SKILL.md",
                "SKILL:current",
                'a',
                'b',
                Some("retirement"),
            ),
            route(
                "route-b-compat",
                "LEGACY-SKILL:compat-second",
                "skills/compat-second/SKILL.md",
                "SKILL:current",
                'c',
                'b',
                Some("compatibility"),
            ),
        ],
        '0',
    );
    let plan = derive_plan(&input).unwrap();
    assert_eq!(plan.effects()[0].route_id(), "route-a-retire");
    assert_eq!(plan.effects()[1].route_id(), "route-b-compat");
    let store = FakeStore::default();
    let mut source = FakeSource::new(input);
    let mut authority = FakeAuthority::current(&plan, 'a');
    authority.cross_deadline_after_captures(3, 86_402_000);
    let token = issue_apply_authorization(&plan, &mut source, &mut authority, &store).unwrap();
    let mut effects = FakeEffects::for_plan(&plan);
    let outcome =
        apply_product_plan(&plan, &token, &mut source, &authority, &store, &mut effects).unwrap();
    assert_eq!(outcome.status(), ApplyOutcomeStatus::RolledBack);
    assert_eq!(effects.counts(), (1, 1));
    assert_eq!(
        effects.authority(plan.effects()[0].effect_id()),
        plan.effects()[0].before().clone()
    );
    assert_eq!(
        effects.authority(plan.effects()[1].effect_id()),
        plan.effects()[1].before().clone()
    );
    assert!(effects.compatibility_prerequisite_inputs().is_empty());
}

#[test]
fn concurrent_compatibility_authorizations_produce_one_bound_effect() {
    let input = compatibility_input();
    let plan = derive_plan(&input).unwrap();
    let store = Arc::new(FakeStore::default());
    let shared_effects = FakeEffects::for_plan(&plan);
    let mut source_a = FakeSource::new(input.clone());
    let mut source_b = FakeSource::new(input);
    let mut authority_a = FakeAuthority::current(&plan, '6');
    let mut authority_b = FakeAuthority::current(&plan, '7');
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
    assert_eq!(shared_effects.counts(), (1, 0));
    assert_eq!(shared_effects.compatibility_prerequisite_inputs().len(), 1);
}

#[test]
fn prerequisite_omission_after_authorization_rejects_without_effect() {
    for field in [
        "owner_id",
        "semantic_target_id",
        "user_facing_warning",
        "usage_measurement",
        "boundary",
        "removal_condition",
    ] {
        let input = compatibility_input();
        let plan = derive_plan(&input).unwrap();
        let store = FakeStore::default();
        let mut source = FakeSource::new(input);
        let mut authority = FakeAuthority::current(&plan, 'f');
        let token = issue_apply_authorization(&plan, &mut source, &mut authority, &store).unwrap();

        let mut route = compatibility_route();
        route["transition"]["adopted_effect"]["compatibility_prerequisites"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        source.input = compatibility_input_with_route(route, '0');
        let mut effects = FakeEffects::for_plan(&plan);
        assert_eq!(
            apply_product_plan(&plan, &token, &mut source, &authority, &store, &mut effects,)
                .unwrap_err()
                .code(),
            "migration-product-registry-invalid",
            "post-authorization omission of {field} must fail"
        );
        assert_eq!(effects.counts(), (0, 0));
        assert!(effects.compatibility_prerequisite_inputs().is_empty());
    }
}

fn valid_prerequisite_substitution(field: &str) -> serde_json::Value {
    let mut route = compatibility_route();
    let prerequisites = &mut route["transition"]["adopted_effect"]["compatibility_prerequisites"];
    match field {
        "owner" => prerequisites["owner_id"] = json!("replacement-maintenance-owner"),
        "warning" => {
            prerequisites["user_facing_warning"] =
                json!("This compatibility route is deprecated; use the replacement target.")
        }
        "measurement" => prerequisites["usage_measurement"]["evidence_sha256"] = json!(sha('e')),
        "boundary" => prerequisites["boundary"]["deadline_unix_ms"] = json!(86_403_000),
        "removal-condition" => prerequisites["removal_condition"]["threshold"] = json!(1),
        _ => unreachable!(),
    }
    route
}
