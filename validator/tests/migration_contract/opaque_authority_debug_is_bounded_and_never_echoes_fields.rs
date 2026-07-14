#[test]
fn opaque_authority_debug_is_bounded_and_never_echoes_fields() {
    let inventory = clean_inventory();
    let route = route("LEGACY-SKILL:old", "SKILL:current");
    let mut replacement_authority = TestReplacementAuthority::current(&inventory, &route);
    replacement_authority.reviewer_id = "opaque-replacement-reviewer-marker".to_owned();
    let replacement = evidence(&inventory, &route, &mut replacement_authority);
    let replacement_debug = format!("{replacement:?}");
    assert_eq!(
        replacement_debug,
        "ReplacementEvidence { contents: \"<redacted>\" }"
    );
    for secret in [
        replacement_authority.reviewer_id.as_str(),
        replacement_authority.session_id.as_str(),
        replacement_authority.nonce_sha256.as_str(),
        replacement_authority.old_result_sha256.as_str(),
    ] {
        assert!(!replacement_debug.contains(secret));
    }

    let (plan, _replacement_authority) = plan_with_authority(&inventory);
    let mut review_authority =
        TestReviewAuthority::current(&inventory, Od009Decision::RequestPhysicalDeletion);
    review_authority.reviewer_id = "opaque-retirement-reviewer-marker".to_owned();
    let review = RetirementReview::issue(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut review_authority,
    )
    .unwrap();
    let review_debug = format!("{review:?}");
    assert_eq!(
        review_debug,
        "RetirementReview { contents: \"<redacted>\" }"
    );
    assert!(!review_debug.contains(&review_authority.reviewer_id));
    assert!(!review_debug.contains(&review_authority.session_id));

    let mut effect_authority =
        TestEffectAuthority::current(&inventory, review.effect_scope_sha256());
    effect_authority.principal_id = "opaque-destructive-principal-marker".to_owned();
    let authorization = DestructiveAuthorization::issue(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &review_authority,
        &mut effect_authority,
    )
    .unwrap();
    let authorization_debug = format!("{authorization:?}");
    assert_eq!(
        authorization_debug,
        "DestructiveAuthorization { contents: \"<redacted>\" }"
    );
    assert!(!authorization_debug.contains(&effect_authority.principal_id));
    assert!(!authorization_debug.contains(&effect_authority.session_id));

    let plan_debug = format!("{plan:?}");
    assert!(plan_debug.contains("private_replacement_ledger: \"<redacted>\""));
    assert!(!plan_debug.contains(&test_digest(b"observed-old-behavior-result")));
}

#[test]
fn safe_plan_projection_serialization_never_traverses_private_replacement_authority() {
    let inventory = clean_inventory();
    let (mut plan, _authority) = plan_with_authority(&inventory);
    let pristine_projection = plan.projection();
    assert!(plan.verify_projection(&pristine_projection).is_ok());

    let sentinels = plan.inject_private_serialization_sentinels_for_test();
    let projection = plan.projection();
    assert_eq!(projection, pristine_projection);
    let plan_json = serde_json::to_string(&projection).unwrap();
    let target_json = serde_json::to_string(&plan.targets()[0].projection()).unwrap();
    assert!(plan_json.len() <= 4_096);
    assert!(target_json.len() <= 2_048);
    for sentinel in sentinels {
        assert!(
            !plan_json.contains(&sentinel),
            "plan projection leaked {sentinel}"
        );
        assert!(
            !target_json.contains(&sentinel),
            "target projection leaked {sentinel}"
        );
    }
    for forbidden in [
        "reviewer_id",
        "authority_id",
        "authority_session_id",
        "read_session_id",
        "nonce_sha256",
        "evidence_id",
        "attestation_sha256",
        "consumption_sha256",
        "consumption_binding_sha256",
        "false_pass_control_results",
    ] {
        assert!(!plan_json.contains(forbidden));
        assert!(!target_json.contains(forbidden));
    }

    let roundtrip: MigrationPlanProjection = serde_json::from_str(&plan_json).unwrap();
    assert_eq!(roundtrip, projection);
    assert!(plan.verify_projection(&roundtrip).is_ok());
    assert_eq!(
        plan.verify_current(&inventory).unwrap_err().code(),
        "migration-plan-stale"
    );

    let mut substituted_json: Value = serde_json::from_str(&plan_json).unwrap();
    substituted_json["projection_sha256"] = Value::String(sha('0'));
    let substituted: MigrationPlanProjection = serde_json::from_value(substituted_json).unwrap();
    assert_eq!(
        plan.verify_projection(&substituted).unwrap_err().code(),
        "migration-plan-projection-substituted"
    );
}

#[test]
fn unsafe_special_and_hardlinked_inventory_inputs_are_rejected() {
    for (kind, links, path) in [
        (SurfaceFileKind::Symlink, 1, "skills/old"),
        (SurfaceFileKind::Special, 1, "skills/old"),
        (SurfaceFileKind::Regular, 2, "skills/old"),
        (SurfaceFileKind::Regular, 1, "../escape"),
    ] {
        let source = InventorySurface::observed(InventorySurfaceObservation {
            stable_id: "LEGACY-SKILL:old".to_owned(),
            kind: "skill".to_owned(),
            relative_path: path.to_owned(),
            digest_sha256: sha('a'),
            file_kind: kind,
            link_count: links,
            status: SurfaceStatus::Active,
            active_readers: vec![],
            active_writers: vec![],
            public_routes: vec![],
            generated_outputs: vec![],
        });
        let error = MigrationInventory::new(
            sha('c'),
            sha('d'),
            sha('e'),
            sha('f'),
            vec![
                source,
                surface(
                    "SKILL:current",
                    SurfaceStatus::Active,
                    vec![],
                    vec![],
                    vec![],
                    vec![],
                ),
            ],
        )
        .unwrap_err();
        assert_eq!(error.code(), "migration-surface-input-refused");
    }
}
