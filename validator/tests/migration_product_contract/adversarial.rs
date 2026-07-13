use super::support::*;
use crate::migration::product::{
    apply_product_plan, derive_product_plan, issue_apply_authorization, recover_product_operation,
    AdoptedRegistrySnapshot, ApplyOutcomeStatus, ProductInputSnapshot,
    ProductMigrationPlanProjection,
};
use crate::migration::{InventorySurface, MigrationInventory, SurfaceFileKind, SurfaceStatus};
use serde_json::json;
use std::sync::{Arc, Barrier};
use std::thread;

#[test]
fn fixture_catalog_names_the_positive_security_recovery_and_false_pass_envelope() {
    let fixture: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../fixtures/migration-engine/product-cases.json"
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
fn crossing_after_authorization_but_immediately_before_effect_has_zero_effect_or_target_state_change(
) {
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
    projection_value["compatibility_boundary_binding"]["initial_observation"]
        ["current_product_version"] = json!("0.0.11");
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

#[test]
fn filesystem_alias_special_hardlink_traversal_and_unicode_inputs_are_rejected() {
    for kind in [
        SurfaceFileKind::Directory,
        SurfaceFileKind::Symlink,
        SurfaceFileKind::Special,
    ] {
        let bad = InventorySurface::observed(
            "LEGACY-SKILL:bad",
            "legacy-skill",
            "skills/bad/SKILL.md",
            sha('a'),
            kind,
            1,
            SurfaceStatus::Active,
            vec![],
            vec![],
            vec![],
            vec![],
        );
        assert!(
            MigrationInventory::new(sha('c'), sha('d'), sha('e'), sha('0'), vec![bad]).is_err()
        );
    }
    let hardlink = InventorySurface::observed(
        "LEGACY-SKILL:bad",
        "legacy-skill",
        "skills/bad/SKILL.md",
        sha('a'),
        SurfaceFileKind::Regular,
        2,
        SurfaceStatus::Active,
        vec![],
        vec![],
        vec![],
        vec![],
    );
    assert!(
        MigrationInventory::new(sha('c'), sha('d'), sha('e'), sha('0'), vec![hardlink]).is_err()
    );
    let traversal = InventorySurface::observed(
        "LEGACY-SKILL:bad",
        "legacy-skill",
        "../escape",
        sha('a'),
        SurfaceFileKind::Regular,
        1,
        SurfaceStatus::Active,
        vec![],
        vec![],
        vec![],
        vec![],
    );
    assert!(
        MigrationInventory::new(sha('c'), sha('d'), sha('e'), sha('0'), vec![traversal]).is_err()
    );
    for stable_id in [
        "/LEGACY-SKILL:absolute",
        "LEGACY-SKILL:../escape",
        "LEGACY-SKILL:double//component",
        "LEGACY-SKILL:dot/./component",
        "LEGACY-SKILL:backslash\\component",
        "LEGACY-SKILL:unicodé",
    ] {
        let bad_identity = InventorySurface::observed(
            stable_id,
            "legacy-skill",
            "skills/bad/SKILL.md",
            sha('a'),
            SurfaceFileKind::Regular,
            1,
            SurfaceStatus::Active,
            vec![],
            vec![],
            vec![],
            vec![],
        );
        assert!(MigrationInventory::new(
            sha('c'),
            sha('d'),
            sha('e'),
            sha('0'),
            vec![bad_identity],
        )
        .is_err());
    }

    let registry_payload = registry_bytes(vec![route(
        "route-old-to-current",
        "LEGACY-SKILL:old",
        "skills/old/SKILL.md",
        "SKILL:current",
        'a',
        'b',
        None,
    )]);
    for kind in [
        SurfaceFileKind::Directory,
        SurfaceFileKind::Symlink,
        SurfaceFileKind::Special,
    ] {
        assert!(AdoptedRegistrySnapshot::observed(
            "migration/authority-routes.json",
            kind,
            1,
            sha('f'),
            registry_payload.clone(),
        )
        .is_err());
    }
    assert!(AdoptedRegistrySnapshot::observed(
        "migration/authority-routes.json",
        SurfaceFileKind::Regular,
        2,
        sha('f'),
        registry_payload,
    )
    .is_err());

    let unicode_inventory = inventory(
        vec![surface(
            "LEGACY-SKILL:unicode",
            "legacy-skill",
            "skills/café/SKILL.md",
            'a',
            SurfaceStatus::Active,
            &[],
            &[],
            &[],
            &[],
        )],
        '0',
    );
    let registry = AdoptedRegistrySnapshot::observed(
        "migration/authority-routes.json",
        SurfaceFileKind::Regular,
        1,
        sha('f'),
        registry_bytes(vec![route(
            "route-unicode",
            "LEGACY-SKILL:unicode",
            "skills/cafe/SKILL.md",
            "SKILL:current",
            'a',
            'b',
            None,
        )]),
    )
    .unwrap();
    assert!(ProductInputSnapshot::observed(unicode_inventory, registry).is_err());

    let collision_inventory = inventory(
        vec![
            surface(
                "LEGACY-SKILL:a",
                "legacy-skill",
                "skills/Case/SKILL.md",
                'a',
                SurfaceStatus::Active,
                &[],
                &[],
                &[],
                &[],
            ),
            surface(
                "LEGACY-SKILL:b",
                "legacy-skill",
                "skills/case/SKILL.md",
                'b',
                SurfaceStatus::Candidate,
                &[],
                &[],
                &[],
                &[],
            ),
        ],
        '0',
    );
    let registry = AdoptedRegistrySnapshot::observed(
        "migration/authority-routes.json",
        SurfaceFileKind::Regular,
        1,
        sha('f'),
        registry_bytes(vec![route(
            "route-case",
            "LEGACY-SKILL:a",
            "skills/Case/SKILL.md",
            "LEGACY-SKILL:b",
            'a',
            'b',
            None,
        )]),
    )
    .unwrap();
    assert_eq!(
        ProductInputSnapshot::observed(collision_inventory, registry)
            .unwrap_err()
            .code(),
        "migration-product-inventory-path-collision"
    );
}
