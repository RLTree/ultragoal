#[test]
fn live_adopted_registry_bytes_match_the_supported_parser_contract() {
    let contract = validate_adopted_registry_bytes(include_bytes!(
        "../../../../migration/authority-routes.json"
    ))
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
