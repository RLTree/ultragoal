#[test]
fn fixture_catalog_names_the_positive_security_recovery_and_false_pass_envelope() {
    let fixture: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../fixtures/migration-engine/product-cases.json"
    ))
    .unwrap();
    assert_eq!(fixture["schema_version"], "MigrationProductCases-v1");
    let positive = fixture["positive_cases"].as_array().unwrap();
    let red = fixture["red_cases"].as_array().unwrap();
    for required in [
        "exact-plan",
        "concurrent-semantic-reservation",
        "crash-after-effect",
        "rollback-reverse-order",
        "terminal-zero-authority",
        "compatibility-prerequisites-bound-to-effect",
        "compatibility-boundary-authority-seal",
        "post-authorization-deadline-crossing-zero-effect",
        "post-effect-preboundary-recovery-reconcile",
        "compatibility-boundary-concurrent-reservation",
        "compatibility-boundary-rollback-prior-effect",
        "bounded-usage-measurement",
        "measurable-removal-condition",
    ] {
        assert!(positive.iter().any(|value| value == required));
    }
    for required in [
        "ambiguous-route",
        "authorization-replay",
        "unicode-path",
        "fifo-or-socket",
        "prose-as-proof",
        "green-command-as-proof",
        "missing-compatibility-owner",
        "missing-boundary-authority",
        "deadline-equal-or-crossed",
        "version-equal-or-crossed",
        "boundary-source-substitution",
        "boundary-journal-substitution",
        "effect-permit-substitution",
        "ambiguous-completion-timing",
        "compatibility-prerequisite-substitution",
        "post-authorization-prerequisite-omission",
        "journal-prerequisite-substitution",
        "physical-deletion-request",
    ] {
        assert!(red.iter().any(|value| value == required));
    }
    assert_eq!(
        fixture["claim_ceiling"],
        "Candidate source-local migration and retirement production runtime only. No claim of root registry adoption, public command availability, actual route retirement, installed runtime behavior, representative live-user journeys, claim elevation, node closure, readiness, release, or completion."
    );
}

#[test]
fn unknown_ambiguous_and_parallel_routes_fail_closed() {
    let surfaces = vec![
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
    ];
    let unknown_source = input_with_routes(
        surfaces.clone(),
        vec![route(
            "route-missing",
            "LEGACY-SKILL:missing",
            "skills/missing/SKILL.md",
            "SKILL:current",
            'a',
            'b',
            None,
        )],
        '0',
    );
    assert_eq!(
        derive_plan(&unknown_source).unwrap_err().code(),
        "migration-product-route-source-unknown"
    );

    let unknown_target = input_with_routes(
        surfaces.clone(),
        vec![route(
            "route-unknown-target",
            "LEGACY-SKILL:old",
            "skills/old/SKILL.md",
            "SKILL:missing",
            'a',
            'b',
            None,
        )],
        '0',
    );
    assert_eq!(
        derive_plan(&unknown_target).unwrap_err().code(),
        "migration-product-route-target-unknown"
    );

    let ambiguous = input_with_routes(
        surfaces.clone(),
        vec![
            route(
                "route-a",
                "LEGACY-SKILL:old",
                "skills/old/SKILL.md",
                "SKILL:current",
                'a',
                'b',
                None,
            ),
            route(
                "route-b",
                "LEGACY-SKILL:old",
                "skills/old/SKILL.md",
                "SKILL:current",
                'a',
                'b',
                None,
            ),
        ],
        '0',
    );
    assert_eq!(
        derive_plan(&ambiguous).unwrap_err().code(),
        "migration-product-route-source-ambiguous"
    );

    let parallel = input_with_routes(
        vec![
            surfaces[0].clone(),
            surface(
                "SKILL:current",
                "skill",
                "skills/current/SKILL.md",
                'b',
                SurfaceStatus::Active,
                &[],
                &[],
                &["current-public"],
                &[],
            ),
        ],
        vec![route(
            "route-parallel",
            "LEGACY-SKILL:old",
            "skills/old/SKILL.md",
            "SKILL:current",
            'a',
            'b',
            None,
        )],
        '0',
    );
    assert_eq!(
        derive_plan(&parallel).unwrap_err().code(),
        "migration-product-active-parallel-authority"
    );
}
