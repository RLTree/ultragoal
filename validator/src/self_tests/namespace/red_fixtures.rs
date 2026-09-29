use serde_json::json;

#[test]
fn namespace_source_topology_is_selective_about_prefixes_and_generic_leaves() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let manifest_paths = vec![
        "validator/src/a_b.rs".to_string(),
        "validator/src/self_tests/domain/helpers.rs".to_string(),
    ];
    let failures = crate::audit::namespace::source::topology::failures(&root, &manifest_paths);
    assert!(
        !failures
            .iter()
            .any(|item| item.contains("namespace_validator_source_residual_prefix_encoding")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|item| item.contains("namespace_validator_source_generic_leaf")),
        "{failures:?}"
    );
}

#[test]
fn namespace_class_registry_rejects_invalid_surface_globs() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let failures = crate::audit::namespace::law::class_registry_value_failures(
        &root,
        &json!({
            "schema":"harness-ultragoal.namespace-class-registry.v1",
            "classes":[{
                "id":"bad-glob",
                "kind":"repo_source",
                "surface_globs":["/absolute/path"],
                "description":"invalid path shape",
                "authority":"validator_source_topology",
                "authority_path":"plugin-manifest-draft.json",
                "maximal_factoring_required":true,
                "waiver_allowed":false,
                "claim_ceiling_impact":"classifies_surface_without_raising_claim_ceiling"
            }]
        }),
    );
    assert!(
        failures
            .iter()
            .any(|item| item.contains("namespace_class_surface_invalid:bad-glob")),
        "{failures:?}"
    );
}
