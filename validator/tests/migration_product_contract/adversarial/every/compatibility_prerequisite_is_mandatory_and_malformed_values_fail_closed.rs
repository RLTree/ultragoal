#[test]
fn every_compatibility_prerequisite_is_mandatory_and_malformed_values_fail_closed() {
    for field in [
        "owner_id",
        "semantic_target_id",
        "user_facing_warning",
        "usage_measurement",
        "boundary",
        "removal_condition",
    ] {
        let mut route = compatibility_route();
        route["transition"]["adopted_effect"]["compatibility_prerequisites"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        let input = compatibility_input_with_route(route, '0');
        assert_eq!(
            derive_plan(&input).unwrap_err().code(),
            "migration-product-registry-invalid",
            "missing prerequisite {field} must be rejected"
        );
    }

    for field in [
        "owner",
        "semantic-target",
        "warning",
        "measurement",
        "boundary",
        "removal-condition",
    ] {
        let mut route = compatibility_route();
        let prerequisites =
            &mut route["transition"]["adopted_effect"]["compatibility_prerequisites"];
        match field {
            "owner" => prerequisites["owner_id"] = json!("owner with spaces"),
            "semantic-target" => prerequisites["semantic_target_id"] = json!("SKILL:other"),
            "warning" => prerequisites["user_facing_warning"] = json!("Use another route"),
            "measurement" => {
                prerequisites["usage_measurement"]["evidence_sha256"] = json!("sha256:invalid")
            }
            "boundary" => prerequisites["boundary"]["deadline_unix_ms"] = json!(2_000),
            "removal-condition" => {
                prerequisites["removal_condition"]["required_consecutive_windows"] = json!(0)
            }
            _ => unreachable!(),
        }
        let input = compatibility_input_with_route(route, '0');
        assert_eq!(
            derive_plan(&input).unwrap_err().code(),
            "migration-product-compatibility-adoption-refused",
            "malformed prerequisite {field} must be rejected"
        );
    }
}

#[test]
fn compatibility_prerequisite_unknown_fields_and_boundary_ambiguity_are_rejected() {
    let mut unknown = compatibility_route();
    unknown["transition"]["adopted_effect"]["compatibility_prerequisites"]
        .as_object_mut()
        .unwrap()
        .insert("unreviewed_escape".to_owned(), json!(true));
    assert_eq!(
        derive_plan(&compatibility_input_with_route(unknown, '0'))
            .unwrap_err()
            .code(),
        "migration-product-registry-invalid"
    );

    let mut ambiguous = compatibility_route();
    ambiguous["transition"]["adopted_effect"]["compatibility_prerequisites"]["boundary"]
        .as_object_mut()
        .unwrap()
        .insert("product_version".to_owned(), json!("0.0.13"));
    assert_eq!(
        derive_plan(&compatibility_input_with_route(ambiguous, '0'))
            .unwrap_err()
            .code(),
        "migration-product-compatibility-adoption-refused"
    );
}

#[test]
fn compatibility_warning_measurement_boundary_and_removal_bounds_reject_overflow() {
    let cases = [
        "warning-bytes",
        "measurement-window",
        "measurement-count",
        "deadline-horizon",
        "removal-threshold",
        "removal-windows",
    ];
    for case in cases {
        let mut route = compatibility_route();
        let prerequisites =
            &mut route["transition"]["adopted_effect"]["compatibility_prerequisites"];
        match case {
            "warning-bytes" => {
                prerequisites["user_facing_warning"] =
                    json!(format!("deprecated {}", "x".repeat(512)))
            }
            "measurement-window" => {
                prerequisites["usage_measurement"]["window_end_unix_ms"] =
                    json!(90_u64 * 24 * 60 * 60 * 1_000 + 1_001)
            }
            "measurement-count" => {
                prerequisites["usage_measurement"]["observed_invocations"] =
                    json!(1_000_000_001_u64)
            }
            "deadline-horizon" => {
                prerequisites["boundary"]["deadline_unix_ms"] =
                    json!(366_u64 * 24 * 60 * 60 * 1_000 + 2_001)
            }
            "removal-threshold" => {
                prerequisites["removal_condition"]["threshold"] = json!(1_000_000_001_u64)
            }
            "removal-windows" => {
                prerequisites["removal_condition"]["required_consecutive_windows"] = json!(53)
            }
            _ => unreachable!(),
        }
        assert_eq!(
            derive_plan(&compatibility_input_with_route(route, '0'))
                .unwrap_err()
                .code(),
            "migration-product-compatibility-adoption-refused",
            "unbounded compatibility prerequisite {case} must be rejected"
        );
    }
}

#[test]
fn compatibility_plan_requires_trusted_boundary_authority_and_rejects_equal_or_crossed_state() {
    let input = compatibility_input();
    assert_eq!(
        derive_product_plan(&input, None).unwrap_err().code(),
        "migration-product-compatibility-boundary-authority-required"
    );

    for now in [86_402_000_u64, 86_402_001] {
        let mut authority = FakeAuthority::boundary();
        authority.boundary_now = now;
        assert_eq!(
            derive_product_plan(&input, Some(&authority))
                .unwrap_err()
                .code(),
            "migration-product-compatibility-boundary-crossed"
        );
    }

    let mut route = compatibility_route();
    let boundary = route["transition"]["adopted_effect"]["compatibility_prerequisites"]["boundary"]
        .as_object_mut()
        .unwrap();
    boundary.remove("deadline_unix_ms");
    boundary.insert("product_version".to_owned(), json!("0.0.13"));
    let input = compatibility_input_with_route(route, '0');
    for version in ["0.0.13", "0.0.14"] {
        let mut authority = FakeAuthority::boundary();
        authority.current_product_version = version.to_owned();
        assert_eq!(
            derive_product_plan(&input, Some(&authority))
                .unwrap_err()
                .code(),
            "migration-product-compatibility-boundary-crossed"
        );
    }
}
