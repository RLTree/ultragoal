use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

const IDS: &[&str] = &[
    "namespace-validator-source-top-level-internal-coverage-wave-red",
    "namespace-validator-source-top-level-internal-cluster-red",
    "namespace-validator-source-iinternal-typo-red",
    "namespace-validator-source-class-waiver-manifest-contract-red",
    "namespace-validator-source-broad-glob-red",
    "namespace-validator-source-generated-class-red",
    "namespace-validator-source-manifest-listing-cannot-bless-red",
    "namespace-validator-source-coverage-wave-history-red",
    "namespace-validator-source-error-not-hidden-by-other-proof-red",
    "namespace-validator-source-widened-class-tamper-red",
];

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

#[test]
fn validator_source_namespace_red_packets_fail_for_intended_errors() {
    let repo = crate::self_tests::boundaries::support::repo_root();
    let root = crate::self_tests::boundaries::support::temp_root("namespace-red-packets");
    std::fs::create_dir_all(root.join("validator/src/self_tests/coverage"))
        .expect("validator source dirs");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    write_json(
        &root.join("docs/namespace-class-registry.json"),
        &json!({
            "schema":"harness-ultragoal.namespace-class-registry.v1",
            "classes":[{
                "id":"repo-source",
                "kind":"repo_source",
                "surface_globs":["validator/src/**/*.rs"],
                "description":"test registry",
                "authority":"validator_source_topology",
                "authority_path":"plugin-manifest-draft.json",
                "maximal_factoring_required":true,
                "waiver_allowed":false,
                "claim_ceiling_impact":"classifies_surface_without_raising_claim_ceiling"
            }]
        }),
    );
    let mut catalog = Vec::new();
    for id in IDS {
        let rel = format!("fixtures/red/{id}.json");
        let packet = crate::json_boundary::read_json(&repo.join(&rel)).expect("packet");
        write_json(&root.join(&rel), &packet);
        catalog.push(json!({
            "id": id,
            "packet_path": rel,
            "expected_failure": packet["expected_failure"]
        }));
    }
    write_json(
        &root.join("templates/RED_FIXTURES.json"),
        &Value::Array(catalog),
    );
    let store = crate::schema_catalog::load(&repo);
    let results = crate::red::fixtures::red_fixture_results(&root, &store, &BTreeMap::new());
    for id in IDS {
        let row = &results[*id];
        assert_eq!(row["status"], "pass", "{id}: {row}");
    }
    std::fs::remove_dir_all(root).expect("cleanup namespace red packets");
}

#[test]
fn namespace_source_topology_is_selective_about_prefixes_and_generic_leaves() {
    let root = crate::self_tests::boundaries::support::repo_root();
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
    let root = crate::self_tests::boundaries::support::repo_root();
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
