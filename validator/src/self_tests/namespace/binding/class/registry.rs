use super::super::{contains, write_text};
use serde_json::json;

#[test]
fn namespace_class_registry_has_no_repeated_prefix_allowance() {
    let root = crate::self_tests::boundaries::support::temp_root("namespace-class-no-waiver");
    std::fs::create_dir_all(&root).expect("namespace class root");
    let registry_failures =
        crate::audit::namespace::law::class_registry_value_failures(&root, &json!({}));
    for expected in [
        "namespace_class_registry_schema_invalid",
        "namespace_class_registry_classes_missing",
    ] {
        assert!(
            contains(&registry_failures, expected),
            "{expected}: {registry_failures:?}"
        );
    }
    let failures = crate::audit::namespace::law::value_failures(
        &root,
        &json!({"resources":[
            "validator/src/domain/schema_alpha.rs",
            "validator/src/domain/schema_beta.rs",
            "validator/tests/domain/schema_gamma.rs"
        ]}),
    );
    assert!(
        contains(
            &failures,
            "namespace_validator_source_residual_prefix_encoding"
        ),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup namespace class no waiver");
}

#[test]
fn namespace_class_registry_requires_exactly_one_class_for_each_path() {
    let registry = json!({"schema":"harness-ultragoal.namespace-class-registry.v1","classes":[
        {"id":"docs","kind":"documentation","description":"docs","authority":"docs","claim_ceiling_impact":"classifies_surface_without_raising_claim_ceiling","surface_globs":["docs/**"],"maximal_factoring_required":true,"waiver_allowed":false,"authority_path":"docs/index.json"},
        {"id":"docs-receipts","kind":"generated_artifact","description":"bad overlap","authority":"receipts","claim_ceiling_impact":"classifies_surface_without_raising_claim_ceiling","surface_globs":["docs/receipts/**"],"maximal_factoring_required":true,"waiver_allowed":false,"authority_path":"docs/index.json"}
    ]});
    let failures = crate::audit::namespace::classes::resolution_failures(
        &registry,
        &[
            "docs/receipts/current.json".to_string(),
            "fixtures/red/bad.json".to_string(),
        ],
    );
    assert!(
        contains(
            &failures,
            "namespace_class_resolution_ambiguous:docs/receipts/current.json"
        ),
        "{failures:?}"
    );
    assert!(
        contains(
            &failures,
            "namespace_class_resolution_missing:fixtures/red/bad.json"
        ),
        "{failures:?}"
    );
}

#[test]
fn namespace_class_registry_accepts_one_class_and_rejects_remaining_loopholes() {
    let root = crate::self_tests::boundaries::support::temp_root("namespace-class-edge-forms");
    write_text(&root.join("authority.json"), "{}");
    let registry = json!({
        "schema":"harness-ultragoal.namespace-class-registry.v1",
        "classes":[
            {"id":"exact","kind":"documentation","description":"exact path","authority":"docs","claim_ceiling_impact":"classifies_surface_without_raising_claim_ceiling","surface_globs":["README.md"],"maximal_factoring_required":true,"waiver_allowed":false,"authority_path":"authority.json"},
            {"id":"tree","kind":"template","description":"tree path","authority":"templates","claim_ceiling_impact":"classifies_surface_without_raising_claim_ceiling","surface_globs":["templates/**"],"maximal_factoring_required":true,"waiver_allowed":false,"authority_path":"authority.json"},
            {"id":"suffix","kind":"documentation","description":"suffix path","authority":"docs","claim_ceiling_impact":"classifies_surface_without_raising_claim_ceiling","surface_globs":["docs/**/*.md"],"maximal_factoring_required":true,"waiver_allowed":false,"authority_path":"authority.json"},
            {"id":"middle","kind":"fixture_catalog","description":"middle path","authority":"fixtures","claim_ceiling_impact":"classifies_surface_without_raising_claim_ceiling","surface_globs":["fixtures/**/receipt.json"],"maximal_factoring_required":true,"waiver_allowed":false,"authority_path":"authority.json"},
            {"id":"bad-max","kind":"documentation","description":"bad max factoring","authority":"docs","claim_ceiling_impact":"classifies_surface_without_raising_claim_ceiling","surface_globs":["bad-max/**"],"maximal_factoring_required":false,"waiver_allowed":false,"authority_path":"authority.json"},
            {"id":"bad-generator","kind":"documentation","description":"bad generator loophole","authority":"docs","claim_ceiling_impact":"classifies_surface_without_raising_claim_ceiling","surface_globs":["bad-generator/**"],"maximal_factoring_required":true,"waiver_allowed":false,"authority_path":"authority.json","generator_or_catalog_path":"docs/namespace-law-exceptions.json"}
        ]
    });
    let failures = crate::audit::namespace::law::class_registry_value_failures(&root, &registry);
    assert!(
        contains(&failures, "namespace_class_untyped:bad-max"),
        "{failures:?}"
    );
    assert!(
        contains(
            &failures,
            "namespace_class_waiver_field_present:bad-generator:generator_or_catalog_path"
        ),
        "{failures:?}"
    );
    let resolution = crate::audit::namespace::classes::resolution_failures(
        &registry,
        &[
            "README.md".to_string(),
            "templates/agent/skill.md".to_string(),
            "docs/a/b.md".to_string(),
            "fixtures/red/receipt.json".to_string(),
            "unknown.txt".to_string(),
        ],
    );
    assert!(
        !resolution
            .iter()
            .any(|failure| failure.contains("README.md")
                || failure.contains("templates/agent/skill.md")
                || failure.contains("docs/a/b.md")
                || failure.contains("fixtures/red/receipt.json")),
        "{resolution:?}"
    );
    assert!(
        contains(
            &resolution,
            "namespace_class_resolution_missing:unknown.txt"
        ),
        "{resolution:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup namespace class edge forms");
}

#[test]
fn namespace_class_registry_rejects_waiver_and_source_loopholes() {
    let root = crate::self_tests::boundaries::support::temp_root("namespace-source-class");
    write_text(&root.join("plugin-manifest-draft.json"), "{}");
    let value = json!({
      "schema":"harness-ultragoal.namespace-class-registry.v1",
      "classes":[
        {
            "id":"bad-source-public",
            "kind":"repo_source",
            "surface_globs":["validator/src/internal*"],
            "description":"class row cannot carry legacy waiver fields",
            "authority":"validator_source_topology",
            "authority_path":"plugin-manifest-draft.json",
            "maximal_factoring_required":true,
            "waiver_allowed":true,
            "exception_type":"repeated_prefix",
            "directory":"validator/src",
            "prefix":"internal",
            "applies_to":["validator/src/internal*"],
            "claim_ceiling_impact":"classifies_surface_without_raising_claim_ceiling"
        },
        {
            "id":"bad-source-generated",
            "kind":"fixture_catalog",
            "surface_globs":["validator/src/domain.rs"],
            "description":"fixture catalog cannot cover hand-authored source",
            "authority":"red_fixture_catalog",
            "authority_path":"plugin-manifest-draft.json",
            "maximal_factoring_required":true,
            "waiver_allowed":false,
            "claim_ceiling_impact":"classifies_surface_without_raising_claim_ceiling"
        }
    ]});
    let failures = crate::audit::namespace::law::class_registry_value_failures(&root, &value);
    for expected in [
        "namespace_class_untyped:bad-source-public",
        "namespace_class_waiver_field_present:bad-source-public:exception_type",
        "namespace_class_broad_source_glob_requires_direct_topology_check:bad-source-public",
        "namespace_class_generated_for_hand_authored_source:bad-source-generated",
    ] {
        assert!(contains(&failures, expected), "{expected}: {failures:?}");
    }
    std::fs::remove_dir_all(root).expect("cleanup namespace source class");
}
