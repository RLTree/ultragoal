#[test]
fn duplicate_reader_writer_public_and_generated_authority_is_rejected() {
    for category in 0..4 {
        let mut first = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        let mut second = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        first[category].push("duplicate");
        second[category].push("duplicate");
        let input = input_with_routes(
            vec![
                surface(
                    "LEGACY-SKILL:old",
                    "legacy-skill",
                    "skills/old/SKILL.md",
                    'a',
                    SurfaceStatus::Active,
                    &first[0],
                    &first[1],
                    &first[2],
                    &first[3],
                ),
                surface(
                    "SKILL:current",
                    "skill",
                    "skills/current/SKILL.md",
                    'b',
                    SurfaceStatus::Active,
                    &second[0],
                    &second[1],
                    &second[2],
                    &second[3],
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
            "migration-product-active-duplicate-authority"
        );
    }
}

#[test]
fn prose_receipts_tests_generated_rows_and_green_commands_never_become_retirement_proof() {
    let mut unadopted = route(
        "route-old-to-current",
        "LEGACY-SKILL:old",
        "skills/old/SKILL.md",
        "SKILL:current",
        'a',
        'b',
        None,
    );
    unadopted["transition"]["equivalence_proof"] = json!("executed-behavior-v1");
    unadopted["transition"]["proof_refs"] = json!([
        "docs/prose.md",
        "generated/row.json",
        "receipts/green-command.json",
        "tests/passing.rs"
    ]);
    let input = input_with_routes(
        vec![
            surface(
                "LEGACY-SKILL:old",
                "legacy-skill",
                "skills/old/SKILL.md",
                'a',
                SurfaceStatus::Active,
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
                SurfaceStatus::Definition,
                &[],
                &[],
                &[],
                &[],
            ),
        ],
        vec![unadopted],
        '0',
    );
    let plan = derive_plan(&input).unwrap();
    assert!(plan.effects().is_empty());
    let store = FakeStore::default();
    let mut source = FakeSource::new(input);
    let mut authority = FakeAuthority::current(&plan, 'a');
    assert_eq!(
        issue_apply_authorization(&plan, &mut source, &mut authority, &store)
            .unwrap_err()
            .code(),
        "migration-product-plan-has-no-adopted-effects"
    );
}

#[test]
fn missing_false_pass_control_and_physical_deletion_authority_are_refused() {
    let surfaces = retirement_input().inventory().surfaces().to_vec();
    let mut missing_control = route(
        "route-old-to-current",
        "LEGACY-SKILL:old",
        "skills/old/SKILL.md",
        "SKILL:current",
        'a',
        'b',
        Some("retirement"),
    );
    missing_control["transition"]["adopted_effect"]["false_pass_control_sha256"]
        .as_object_mut()
        .unwrap()
        .remove("receipt-production");
    let input = input_with_routes(surfaces.clone(), vec![missing_control], '0');
    assert_eq!(
        derive_plan(&input).unwrap_err().code(),
        "migration-product-adopted-transition-invalid"
    );

    let mut bytes: serde_json::Value = serde_json::from_slice(&registry_bytes(vec![route(
        "route-old-to-current",
        "LEGACY-SKILL:old",
        "skills/old/SKILL.md",
        "SKILL:current",
        'a',
        'b',
        Some("retirement"),
    )]))
    .unwrap();
    bytes["destructive_cleanup_authorized"] = json!(true);
    let registry = AdoptedRegistrySnapshot::observed(
        "migration/authority-routes.json",
        SurfaceFileKind::Regular,
        1,
        sha('f'),
        serde_json::to_vec(&bytes).unwrap(),
    )
    .unwrap();
    let input = ProductInputSnapshot::observed(inventory(surfaces, '0'), registry).unwrap();
    assert_eq!(
        derive_plan(&input).unwrap_err().code(),
        "migration-product-registry-contract-refused"
    );
}
