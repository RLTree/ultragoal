#[test]
fn every_prerequisite_substitution_changes_plan_effect_and_sealed_authorization_identity() {
    for field in [
        "owner",
        "warning",
        "measurement",
        "boundary",
        "removal-condition",
    ] {
        let base_input = compatibility_input();
        let base_plan = derive_plan(&base_input).unwrap();
        let base_store = FakeStore::default();
        let mut base_source = FakeSource::new(base_input);
        let mut base_authority = FakeAuthority::current(&base_plan, 'a');
        let base_token = issue_apply_authorization(
            &base_plan,
            &mut base_source,
            &mut base_authority,
            &base_store,
        )
        .unwrap();

        let substituted_input =
            compatibility_input_with_route(valid_prerequisite_substitution(field), '0');
        let substituted_plan = derive_plan(&substituted_input).unwrap();
        assert_ne!(base_plan.plan_sha256(), substituted_plan.plan_sha256());
        assert_ne!(
            base_plan.effects()[0].effect_id(),
            substituted_plan.effects()[0].effect_id()
        );
        assert_ne!(
            base_plan.effects()[0].compatibility_prerequisites_sha256(),
            substituted_plan.effects()[0].compatibility_prerequisites_sha256()
        );

        let substituted_store = FakeStore::default();
        let mut substituted_source = FakeSource::new(substituted_input.clone());
        let mut substituted_authority = FakeAuthority::current(&substituted_plan, 'a');
        let substituted_token = issue_apply_authorization(
            &substituted_plan,
            &mut substituted_source,
            &mut substituted_authority,
            &substituted_store,
        )
        .unwrap();
        assert_ne!(
            base_token.authorization_id(),
            substituted_token.authorization_id(),
            "sealed authorization must bind prerequisite {field}"
        );

        base_source.input = substituted_input;
        let mut effects = FakeEffects::for_plan(&base_plan);
        assert_eq!(
            apply_product_plan(
                &base_plan,
                &base_token,
                &mut base_source,
                &base_authority,
                &base_store,
                &mut effects,
            )
            .unwrap_err()
            .code(),
            "migration-product-apply-input-stale"
        );
        assert_eq!(effects.counts(), (0, 0));
        assert!(effects.compatibility_prerequisite_inputs().is_empty());
    }
}

#[test]
fn durable_journal_prerequisite_mutation_or_omission_rejects_before_effect() {
    for mutation in ["substitute-owner", "omit-prerequisites"] {
        let input = compatibility_input();
        let plan = derive_plan(&input).unwrap();
        let store = FakeStore::default();
        store.fail_on_cas(1);
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
        assert_eq!(effects.counts(), (0, 0));

        let interrupted = store.only_operation();
        let operation_id = interrupted.operation_id().to_owned();
        let mut value = serde_json::to_value(interrupted).unwrap();
        match mutation {
            "substitute-owner" => {
                value["effects"][0]["compatibility_prerequisites"]["owner_id"] =
                    json!("substituted-owner")
            }
            "omit-prerequisites" => {
                value["effects"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("compatibility_prerequisites");
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
        assert_eq!(effects.counts(), (0, 0));
        assert!(effects.compatibility_prerequisite_inputs().is_empty());
    }
}

#[test]
fn exact_terminal_projection_rejects_duplicate_authority_outside_the_retired_source() {
    let input = input_with_routes(
        vec![
            surface(
                "LEGACY-SKILL:old",
                "legacy-skill",
                "skills/old/SKILL.md",
                'a',
                SurfaceStatus::Candidate,
                &[],
                &[],
                &[],
                &[],
            ),
            surface(
                "SKILL:current",
                "skill",
                "skills/current/SKILL.md",
                'b',
                SurfaceStatus::Active,
                &["duplicate-terminal-reader"],
                &[],
                &[],
                &[],
            ),
            surface(
                "SKILL:unrelated",
                "skill",
                "skills/unrelated/SKILL.md",
                'c',
                SurfaceStatus::Candidate,
                &["duplicate-terminal-reader"],
                &[],
                &[],
                &[],
            ),
        ],
        vec![route(
            "route-old-to-current",
            "LEGACY-SKILL:old",
            "skills/old/SKILL.md",
            "SKILL:current",
            'a',
            'b',
            Some("retirement"),
        )],
        '0',
    );
    assert_eq!(
        derive_plan(&input).unwrap_err().code(),
        "migration-product-terminal-duplicate-authority"
    );
}
