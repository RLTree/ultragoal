use super::support::*;
use crate::migration::product::{
    apply_product_plan, derive_product_plan, issue_apply_authorization,
    validate_adopted_registry_bytes, ApplyOutcomeStatus, PlanDisposition,
};
use crate::migration::SurfaceStatus;

#[test]
fn live_adopted_registry_bytes_match_the_supported_parser_contract() {
    let contract =
        validate_adopted_registry_bytes(include_bytes!("../../../migration/authority-routes.json"))
            .unwrap();
    assert_eq!(contract, "harness-ultragoal-successor-contract-v2");
}

#[test]
fn path_bearing_live_registry_identifiers_form_an_exact_plan() {
    let source_id = "LEGACY-COMMAND:validator/src/command/mod.rs";
    let input = input_with_routes(
        vec![
            surface(
                source_id,
                "legacy-skill",
                "validator/src/command/mod.rs",
                'a',
                SurfaceStatus::Candidate,
                &["legacy-command-reader"],
                &[],
                &[],
                &[],
            ),
            surface(
                "PS-CLI",
                "product-surface",
                "validator/src/product/cli/mod.rs",
                'b',
                SurfaceStatus::Active,
                &[],
                &[],
                &["ps-cli"],
                &[],
            ),
        ],
        vec![route(
            "command-root-to-ps-cli",
            source_id,
            "validator/src/command/mod.rs",
            "PS-CLI",
            'a',
            'b',
            Some("retirement"),
        )],
        '0',
    );
    let plan = derive_plan(&input).unwrap();
    assert_eq!(plan.effects().len(), 1);
    assert_eq!(plan.effects()[0].before().stable_id(), source_id);
}

#[test]
fn exact_plan_classifies_sole_legacy_plus_definition_as_pending() {
    let input = input_with_routes(
        vec![
            surface(
                "LEGACY-SKILL:old",
                "legacy-skill",
                "skills/old/SKILL.md",
                'a',
                SurfaceStatus::Active,
                &["reader-a"],
                &["writer-a"],
                &["public-a"],
                &["generated-a"],
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
        vec![route(
            "route-old-to-current",
            "LEGACY-SKILL:old",
            "skills/old/SKILL.md",
            "SKILL:current",
            'a',
            'b',
            None,
        )],
        '0',
    );
    let first = derive_plan(&input).unwrap();
    let second = derive_plan(&input).unwrap();
    assert_eq!(first.plan_sha256(), second.plan_sha256());
    assert!(first.effects().is_empty());
    let value = serde_json::to_value(first.projection()).unwrap();
    assert_eq!(value["items"][0]["disposition"], "pending_migration");
    assert_eq!(
        value["items"][0]["reason"],
        "sole_legacy_plus_definition_pending_migration"
    );
    assert_eq!(value["items"][0]["exact_active_readers"][0], "reader-a");
    assert_eq!(value["items"][0]["exact_active_writers"][0], "writer-a");
    assert_eq!(value["items"][0]["exact_public_routes"][0], "public-a");
    assert_eq!(
        value["items"][0]["exact_generated_outputs"][0],
        "generated-a"
    );
}

#[test]
fn explicitly_adopted_compatibility_transition_applies_once_and_repeats_idempotently() {
    let input = compatibility_input();
    let plan = derive_plan(&input).unwrap();
    let repeated_plan = derive_plan(&input).unwrap();
    assert_eq!(plan.plan_sha256(), repeated_plan.plan_sha256());
    assert_eq!(
        serde_json::to_vec(&plan.projection()).unwrap(),
        serde_json::to_vec(&repeated_plan.projection()).unwrap()
    );
    assert_eq!(plan.effects().len(), 1);
    assert_eq!(
        plan.effects()[0].disposition(),
        PlanDisposition::AdoptCompatibility
    );
    assert_eq!(
        plan.effects()[0].before().digest_sha256(),
        plan.effects()[0].after().digest_sha256(),
        "OD-009 physical bytes must remain exact"
    );
    let effect_value = serde_json::to_value(&plan.effects()[0]).unwrap();
    let prerequisites = &effect_value["compatibility_prerequisites"];
    assert_eq!(prerequisites["owner_id"], "maintenance-owner");
    assert_eq!(prerequisites["semantic_target_id"], "SKILL:current");
    assert_eq!(
        prerequisites["user_facing_warning"],
        "This compatibility route is deprecated; use the canonical target."
    );
    assert_eq!(
        prerequisites["usage_measurement"]["metric"],
        "legacy-route-invocations"
    );
    assert_eq!(
        prerequisites["usage_measurement"]["observed_invocations"],
        12
    );
    assert_eq!(prerequisites["boundary"]["deadline_unix_ms"], 86_402_000);
    assert_eq!(
        prerequisites["removal_condition"]["operator"],
        "less-than-or-equal"
    );
    assert_eq!(prerequisites["removal_condition"]["threshold"], 0);
    let projection_value = serde_json::to_value(plan.projection()).unwrap();
    let boundary_binding = &projection_value["compatibility_boundary_binding"];
    assert_eq!(
        boundary_binding["initial_observation"]["authority_id"],
        "root-compatibility-boundary-authority"
    );
    assert_eq!(
        boundary_binding["initial_observation"]["source_identity_sha256"],
        sha('b')
    );
    assert_eq!(
        boundary_binding["initial_observation"]["observed_at_unix_ms"],
        10_000
    );
    assert_eq!(
        boundary_binding["initial_observation"]["current_product_version"],
        "0.0.12"
    );
    let prerequisites_sha256 = plan.effects()[0]
        .compatibility_prerequisites_sha256()
        .unwrap()
        .to_owned();

    let store = FakeStore::default();
    let mut source = FakeSource::new(input);
    let mut authority = FakeAuthority::current(&plan, 'a');
    let token = issue_apply_authorization(&plan, &mut source, &mut authority, &store).unwrap();
    let mut effects = FakeEffects::for_plan(&plan);
    let applied =
        apply_product_plan(&plan, &token, &mut source, &authority, &store, &mut effects).unwrap();
    assert_eq!(applied.status(), ApplyOutcomeStatus::Applied);
    assert!(applied.terminal_proof_sha256().is_some());
    assert_eq!(effects.counts(), (1, 0));
    assert_eq!(
        effects.compatibility_prerequisite_inputs(),
        [prerequisites_sha256]
    );

    let repeated =
        apply_product_plan(&plan, &token, &mut source, &authority, &store, &mut effects).unwrap();
    assert_eq!(repeated.status(), ApplyOutcomeStatus::AlreadyApplied);
    assert_eq!(repeated.operation_id(), applied.operation_id());
    assert_eq!(effects.counts(), (1, 0));
}

#[test]
fn compatibility_version_boundary_is_an_explicit_unambiguous_alternative_to_a_deadline() {
    let mut route = compatibility_route();
    let boundary = route["transition"]["adopted_effect"]["compatibility_prerequisites"]["boundary"]
        .as_object_mut()
        .unwrap();
    boundary.remove("deadline_unix_ms");
    boundary.insert("product_version".to_owned(), serde_json::json!("0.0.13"));
    let input = compatibility_input_with_route(route, '0');
    let plan = derive_plan(&input).unwrap();
    let value = serde_json::to_value(&plan.effects()[0]).unwrap();
    assert_eq!(
        value["compatibility_prerequisites"]["boundary"]["product_version"],
        "0.0.13"
    );
    let store = FakeStore::default();
    let mut source = FakeSource::new(input);
    let mut authority = FakeAuthority::current(&plan, 'c');
    let token = issue_apply_authorization(&plan, &mut source, &mut authority, &store).unwrap();
    let mut effects = FakeEffects::for_plan(&plan);
    let outcome =
        apply_product_plan(&plan, &token, &mut source, &authority, &store, &mut effects).unwrap();
    assert_eq!(outcome.status(), ApplyOutcomeStatus::Applied);
    assert_eq!(effects.counts(), (1, 0));
}

#[test]
fn retirement_terminal_requires_zero_reader_writer_public_and_generated_authority() {
    let input = retirement_input();
    let plan = derive_plan(&input).unwrap();
    let effect = &plan.effects()[0];
    assert_eq!(effect.disposition(), PlanDisposition::RetireAuthority);
    assert_eq!(effect.after().status(), SurfaceStatus::Retired);
    assert!(effect.after().active_readers().is_empty());
    assert!(effect.after().active_writers().is_empty());
    assert!(effect.after().public_routes().is_empty());
    assert!(effect.after().generated_outputs().is_empty());
    assert_eq!(
        effect.before().digest_sha256(),
        effect.after().digest_sha256(),
        "retirement removes authority, not OD-009 bytes"
    );

    let store = FakeStore::default();
    let mut source = FakeSource::new(input);
    let mut authority = FakeAuthority::current(&plan, 'b');
    let token = issue_apply_authorization(&plan, &mut source, &mut authority, &store).unwrap();
    let mut effects = FakeEffects::for_plan(&plan);
    let outcome =
        apply_product_plan(&plan, &token, &mut source, &authority, &store, &mut effects).unwrap();
    assert_eq!(outcome.status(), ApplyOutcomeStatus::Applied);
    assert!(outcome.terminal_proof_sha256().is_some());
    let output = outcome.to_canonical_json().unwrap();
    assert!(output.len() < 2 * 1024 * 1024);
    let text = String::from_utf8(output).unwrap();
    assert!(text.contains("source_local_migration_effects_only"));
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        value["claim_ceiling"],
        "source_local_migration_effects_only_not_root_adoption_or_runtime_completion"
    );
}
