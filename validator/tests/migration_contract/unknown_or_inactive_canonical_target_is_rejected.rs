#[test]
fn unknown_or_inactive_canonical_target_is_rejected() {
    let inventory = clean_inventory();
    let route = route("LEGACY-SKILL:old", "SKILL:missing");
    let mut authority = TestReplacementAuthority::current(&inventory, &route);
    let evidence = evidence(&inventory, &route, &mut authority);
    let unknown =
        MigrationPlan::build(&inventory, vec![route], vec![evidence], &mut authority).unwrap_err();
    assert_eq!(unknown.code(), "migration-route-target-unknown");
}

#[test]
fn ambiguous_routes_for_one_source_fail_closed() {
    let inventory = MigrationInventory::new(
        sha('c'),
        sha('d'),
        sha('e'),
        sha('f'),
        vec![
            surface(
                "LEGACY-SKILL:old",
                SurfaceStatus::Active,
                vec![],
                vec![],
                vec![],
                vec![],
            ),
            surface(
                "SKILL:current",
                SurfaceStatus::Active,
                vec![],
                vec![],
                vec![],
                vec![],
            ),
            surface(
                "SKILL:other",
                SurfaceStatus::Active,
                vec![],
                vec![],
                vec![],
                vec![],
            ),
        ],
    )
    .unwrap();
    let second = CompatibilityRoute::new(CompatibilityRouteDefinition {
        route_id: "route-old-to-other".to_owned(),
        source_id: "LEGACY-SKILL:old".to_owned(),
        canonical_target_id: "SKILL:other".to_owned(),
        owner_id: "migration-owner".to_owned(),
        warning: "Compatibility warning for another target".to_owned(),
        usage_measurement_sha256: sha('1'),
        compatibility_boundary: "version-2-boundary".to_owned(),
        removal_condition: "zero-active-references".to_owned(),
        equivalence_sha256: sha('2'),
        observed_invocations: 0,
        compatibility_window_complete: true,
    });
    let first = route("LEGACY-SKILL:old", "SKILL:current");
    let mut first_authority = TestReplacementAuthority::current(&inventory, &first);
    let first_evidence = evidence(&inventory, &first, &mut first_authority);
    let mut second_authority = TestReplacementAuthority::current(&inventory, &second);
    let second_evidence = evidence(&inventory, &second, &mut second_authority);
    let error = MigrationPlan::build(
        &inventory,
        vec![first, second],
        vec![first_evidence, second_evidence],
        &mut first_authority,
    )
    .unwrap_err();
    assert!(matches!(
        error.code(),
        "migration-duplicate-replacement-evidence" | "migration-ambiguous-source-route"
    ));
}

#[test]
fn route_requires_bounded_warning_measurement_and_equivalence() {
    let invalid = CompatibilityRoute::new(CompatibilityRouteDefinition {
        route_id: "route".to_owned(),
        source_id: "LEGACY-SKILL:old".to_owned(),
        canonical_target_id: "SKILL:current".to_owned(),
        owner_id: "owner".to_owned(),
        warning: "silent alias".to_owned(),
        usage_measurement_sha256: "not-a-digest".to_owned(),
        compatibility_boundary: "boundary".to_owned(),
        removal_condition: "removal".to_owned(),
        equivalence_sha256: "not-a-digest".to_owned(),
        observed_invocations: 0,
        compatibility_window_complete: true,
    });
    assert_eq!(
        invalid.validate().unwrap_err().code(),
        "migration-route-warning-invalid"
    );
}

#[test]
fn stale_replacement_evidence_is_rejected_before_planning() {
    let inventory = clean_inventory();
    let stale_route = route("LEGACY-SKILL:old", "SKILL:current");
    let mut authority = TestReplacementAuthority::current(&inventory, &stale_route);
    let mut stale = evidence(&inventory, &stale_route, &mut authority);
    stale.substitute_semantic_for_test(sha('8'));
    assert_eq!(
        MigrationPlan::build(&inventory, vec![stale_route], vec![stale], &mut authority,)
            .unwrap_err()
            .code(),
        "migration-replacement-evidence-stale-or-substituted"
    );

    let route = route("LEGACY-SKILL:old", "SKILL:current");
    let mut authority = TestReplacementAuthority::current(&inventory, &route);
    let mut substituted = evidence(&inventory, &route, &mut authority);
    substituted.substitute_attestation_for_test(sha('9'));
    assert_eq!(
        MigrationPlan::build(&inventory, vec![route], vec![substituted], &mut authority,)
            .unwrap_err()
            .code(),
        "migration-replacement-evidence-stale-or-substituted"
    );
}

#[test]
fn replacement_evidence_requires_every_exact_named_false_pass_control() {
    let inventory = clean_inventory();
    let route = route("LEGACY-SKILL:old", "SKILL:current");
    let mut missing = TestReplacementAuthority::current(&inventory, &route);
    missing.controls.remove("verbosity");
    assert_eq!(
        ReplacementEvidence::issue(&inventory, &route, &mut missing)
            .unwrap_err()
            .code(),
        "migration-replacement-controls-incomplete"
    );

    let mut substituted = TestReplacementAuthority::current(&inventory, &route);
    substituted.controls.insert(
        "generic-negative-control".to_owned(),
        (
            EvidenceVerdict::CausalFailure,
            test_digest(b"generic-control-result"),
        ),
    );
    assert_eq!(
        ReplacementEvidence::issue(&inventory, &route, &mut substituted)
            .unwrap_err()
            .code(),
        "migration-replacement-controls-incomplete"
    );
}

#[test]
fn repeated_hash_replacement_attack_is_rejected_at_observation() {
    let inventory = clean_inventory();
    let route = route("LEGACY-SKILL:old", "SKILL:current");
    let mut authority = TestReplacementAuthority::current(&inventory, &route);
    let repeated = sha('a');
    authority.old_result_sha256 = repeated.clone();
    authority.new_result_sha256 = repeated.clone();
    authority.journey_result_sha256 = repeated.clone();
    authority.rollback_result_sha256 = repeated.clone();
    for (_, digest) in authority.controls.values_mut() {
        *digest = repeated.clone();
    }
    assert_eq!(
        ReplacementEvidence::issue(&inventory, &route, &mut authority)
            .unwrap_err()
            .code(),
        "migration-replacement-repeated-digest-refused"
    );
}
