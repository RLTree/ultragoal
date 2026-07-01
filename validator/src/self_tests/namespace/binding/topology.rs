use super::{contains, write_json, write_text};
use serde_json::json;

#[test]
fn namespace_topology_inspects_validator_source_even_when_unlisted() {
    let root = crate::self_tests::boundaries::support::temp_root("namespace-source-topology");
    write_text(
        &root.join("validator/src/internal_claim_tests.rs"),
        "#[test]\nfn hidden() {}\n",
    );
    let failures = crate::audit::namespace::law::value_failures(
        &root,
        &json!({"resources":["plugin-manifest-draft.json"]}),
    );
    assert!(
        contains(
            &failures,
            "namespace_validator_source_top_level_internal_cluster"
        ),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup namespace source topology");
}

#[test]
fn namespace_topology_rejects_history_names_after_directory_move() {
    let root = crate::self_tests::boundaries::support::temp_root("namespace-history-name");
    write_text(
        &root.join("validator/src/self_tests/coverage/coverage_wave99_tests.rs"),
        "#[test]\nfn still_history() {}\n",
    );
    let failures = crate::audit::namespace::law::value_failures(&root, &json!({"resources":[]}));
    assert!(
        contains(&failures, "namespace_validator_source_history_name"),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup namespace history name");
}

#[test]
fn namespace_topology_rejects_partially_factored_module_roots() {
    let root = crate::self_tests::boundaries::support::temp_root("namespace-partial-module-root");
    write_text(
        &root.join("validator/src/claim_semantics.rs"),
        "pub(crate) mod product;\n",
    );
    write_text(
        &root.join("validator/src/claim_semantics/product.rs"),
        "pub(crate) fn marker() {}\n",
    );
    let failures = crate::audit::namespace::law::value_failures(&root, &json!({"resources":[]}));
    assert!(
        contains(
            &failures,
            "namespace_validator_source_partial_module_factoring"
        ),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup namespace partial module root");
}

#[test]
fn namespace_topology_accepts_semantic_validator_self_test_routes() {
    let root = crate::self_tests::boundaries::support::temp_root("namespace-semantic-green");
    write_text(
        &root.join("validator/src/self_tests/claim/evidence/boundaries.rs"),
        "#[test]\nfn semantic() {}\n",
    );
    write_text(
        &root.join("validator/src/self_tests/coverage/receipt/authority.rs"),
        "#[test]\nfn semantic() {}\n",
    );
    write_text(
        &root.join("validator/src/claim_semantics/mod.rs"),
        "pub(crate) mod product;\n",
    );
    write_text(
        &root.join("validator/src/claim_semantics/product/mod.rs"),
        "pub(crate) fn marker() {}\n",
    );
    let failures = crate::audit::namespace::law::value_failures(&root, &json!({"resources":[]}));
    assert!(
        !contains(&failures, "namespace_validator_source"),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup namespace semantic green");
}

#[test]
fn namespace_legacy_exception_surface_is_hard_failure() {
    let root = crate::self_tests::boundaries::support::temp_root("namespace-legacy-waiver");
    write_json(
        &root.join("docs/namespace-law-exceptions.json"),
        &json!({"schema":"harness-ultragoal.namespace-law-exceptions.v1","exceptions":[]}),
    );
    let failures = crate::audit::namespace::law::value_failures(
        &root,
        &json!({"resources":["docs/namespace-law-exceptions.json"]}),
    );
    for expected in [
        "namespace_waiver_surface_present",
        "namespace_waiver_surface_listed",
    ] {
        assert!(contains(&failures, expected), "{expected}: {failures:?}");
    }
    std::fs::remove_dir_all(root).expect("cleanup namespace legacy waiver");
}

#[test]
fn namespace_topology_rejects_opaque_gate_number_names() {
    let root = crate::self_tests::boundaries::support::temp_root("namespace-gate-number-name");
    write_text(
        &root.join("validator/src/audit/research/trace/gate92.rs"),
        "pub(crate) fn marker() {}\n",
    );
    let failures = crate::audit::namespace::law::value_failures(&root, &json!({"resources":[]}));
    assert!(
        contains(
            &failures,
            "namespace_validator_source_opaque_gate_number_name"
        ),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup namespace gate number name");
}
