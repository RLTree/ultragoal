use super::{contains, write_json, write_text};
use serde_json::json;

#[test]
fn namespace_topology_inspects_validator_source_even_when_unlisted() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("namespace-source-topology");
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
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("namespace-history-name");
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
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "namespace-partial-module-root",
    );
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
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("namespace-semantic-green");
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
fn namespace_topology_factors_only_missing_three_sibling_namespaces() {
    let failures = crate::audit::namespace::source::topology::failures_with_repo_paths(
        &[
            "validator/src/audit/receipt_parse.rs".to_string(),
            "validator/src/audit/receipt_report.rs".to_string(),
            "validator/src/audit/receipt_write.rs".to_string(),
            "validator/src/audit/response_parse.rs".to_string(),
            "validator/src/audit/response_report.rs".to_string(),
            "validator/src/audit/command.rs".to_string(),
            "validator/src/audit/command_parse.rs".to_string(),
            "validator/src/audit/command_report.rs".to_string(),
            "validator/src/audit/command_write.rs".to_string(),
            "validator/tests/receipt_parse.rs".to_string(),
            "validator/tests/receipt_report.rs".to_string(),
            "validator/tests/receipt_write.rs".to_string(),
        ],
        &[],
    );
    assert!(
        failures.iter().any(|failure| {
            failure.contains("namespace_validator_source_residual_prefix_encoding")
                && failure.contains("prefix=receipt")
        }),
        "{failures:?}"
    );
    assert!(
        !failures
            .iter()
            .any(|failure| failure.contains("prefix=command")),
        "{failures:?}"
    );
    assert!(
        !failures
            .iter()
            .any(|failure| failure.contains("prefix=response")),
        "{failures:?}"
    );
    let cargo_entrypoints = crate::audit::namespace::source::topology::failures_with_repo_paths(
        &[
            "validator/tests/receipt_parse.rs".to_string(),
            "validator/tests/receipt_report.rs".to_string(),
            "validator/tests/receipt_write.rs".to_string(),
        ],
        &[],
    );
    assert!(
        !cargo_entrypoints
            .iter()
            .any(|failure| failure.contains("namespace_validator_source_residual_prefix_encoding")),
        "{cargo_entrypoints:?}"
    );
}

#[test]
fn namespace_topology_accepts_prefixes_owned_by_the_containing_directory() {
    let failures = crate::audit::namespace::source::topology::failures_with_repo_paths(
        &[
            "validator/src/observability/event/event_record.rs".to_string(),
            "validator/src/observability/event/event_query.rs".to_string(),
            "validator/src/observability/event/event_publication.rs".to_string(),
        ],
        &[],
    );
    assert!(
        !failures
            .iter()
            .any(|failure| failure.contains("namespace_validator_source_residual_prefix_encoding")),
        "{failures:?}"
    );
}

#[test]
fn namespace_legacy_exception_surface_is_hard_failure() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("namespace-legacy-waiver");
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
