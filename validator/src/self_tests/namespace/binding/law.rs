use super::{contains, write_json};
use serde_json::json;

#[test]
fn namespace_binding_failures_cover_missing_standards_trace_and_red_fixture() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("namespace-binding-failures");
    write_json(
        &root.join("docs/namespace-class-registry.json"),
        &json!({"schema":"harness-ultragoal.namespace-class-registry.v1","classes":[]}),
    );
    let failures = crate::audit::namespace::law::package_failures(&root, &json!({"resources":[]}));
    for expected in [
        "namespace_law_missing_standards_row",
        "namespace_law_missing_foundational_trace",
        "namespace_law_present_only_as_prose",
    ] {
        assert!(contains(&failures, expected), "{expected}: {failures:?}");
    }
    std::fs::remove_dir_all(root).expect("cleanup namespace binding failures");
}

#[test]
fn namespace_value_failures_accepts_fully_listed_files_without_orphans() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("namespace-no-orphans");
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::write(root.join("docs/routed.txt"), "routed").expect("routed file");
    let failures = crate::audit::namespace::law::value_failures(
        &root,
        &json!({"resources":["docs/routed.txt"]}),
    );
    assert!(
        !contains(&failures, "namespace_orphan_repo_file"),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup namespace no orphans");
}

#[test]
fn current_root_namespace_audit_fails_closed_when_inventory_is_unavailable() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "namespace-current-root-inventory-unavailable",
    );
    let _ = std::fs::remove_dir_all(&root);
    let failures = crate::audit::namespace::law::current_root_failures(&root);
    assert!(
        failures.contains(&"namespace_current_root_inventory_unavailable".to_string()),
        "{failures:?}"
    );
    assert!(
        contains(&failures, "governed_source_authority_unavailable"),
        "{failures:?}"
    );
}

#[test]
fn namespace_law_rejects_generic_schema_json_authority_names() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("namespace-schema-json");
    std::fs::create_dir_all(&root).expect("namespace schema root");
    let failures = crate::audit::namespace::law::value_failures(
        &root,
        &json!({"resources":["schemas/common.json"]}),
    );
    assert!(
        contains(&failures, "namespace_schema_generic_authority_path"),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup namespace schema json");
}

#[test]
fn missing_class_registry_does_not_hide_other_namespace_findings() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "namespace-missing-registry-complete-audit",
    );
    std::fs::create_dir_all(&root).expect("namespace root");
    let failures = crate::audit::namespace::law::package_failures(
        &root,
        &json!({"resources":["schemas/common.json"]}),
    );
    assert!(
        contains(&failures, "namespace_class_registry_file_missing"),
        "{failures:?}"
    );
    assert!(
        contains(&failures, "namespace_schema_generic_authority_path"),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup missing registry audit");
}

#[test]
fn namespace_value_cache_reuses_repo_source_paths() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("namespace-value-cache");
    std::fs::create_dir_all(root.join("validator/src/domain")).expect("validator src");
    std::fs::write(root.join("validator/src/domain/leaf.rs"), "fn leaf() {}\n")
        .expect("source leaf");
    let manifest = json!({"resources":["validator/src/domain/leaf.rs"]});
    let mut cache = crate::audit::namespace::law::ValueCache::default();
    let first =
        crate::audit::namespace::law::value_failures_with_cache(&root, &manifest, &mut cache);
    let second =
        crate::audit::namespace::law::value_failures_with_cache(&root, &manifest, &mut cache);
    assert_eq!(first, second);
    std::fs::remove_dir_all(root).expect("cleanup namespace value cache");
}

#[test]
fn source_inventory_findings_do_not_hide_semantic_identifier_findings() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "namespace-complete-source-audit",
    );
    std::fs::create_dir_all(root.join("validator/src/domain")).expect("validator source");
    std::fs::write(
        root.join("validator/src/domain/behavior.rs"),
        "#![allow(dead_code)]\nfn checkpoint_progress() {}\n",
    )
    .expect("source with two independent findings");
    let failures = crate::audit::namespace::law::value_failures(
        &root,
        &json!({"resources":["validator/src/domain/behavior.rs"]}),
    );
    assert!(
        contains(&failures, "plugin_self_law_forbidden_lint_allowance"),
        "{failures:?}"
    );
    assert!(
        contains(
            &failures,
            "namespace_validator_source_product_opaque_goal_work_identifier"
        ),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup complete namespace audit");
}

#[test]
fn namespace_law_detects_mixed_domains_and_accepts_current_red_binding() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("namespace-mixed-domain");
    std::fs::create_dir_all(root.join("runtime-product")).expect("mixed domain dir");
    std::fs::write(root.join("runtime-product/flow.rs"), "flow").expect("mixed domain file");
    write_json(
        &root.join("docs/namespace-class-registry.json"),
        &json!({"schema":"harness-ultragoal.namespace-class-registry.v1","classes":[]}),
    );
    write_json(
        &root.join("templates/agent-standards/enforcement.json"),
        &json!({"rows":[{"id":"namespace-progressive-disclosure"}]}),
    );
    write_json(
        &root.join("docs/foundational-law-traceability.json"),
        &json!({"entries":[{"obligation_id":"namespace-progressive-disclosure"}]}),
    );
    write_json(
        &root.join("templates/RED_FIXTURES.json"),
        &json!([{"id":"namespace-red","expected_failure":{"check_id":"namespace-progressive-disclosure"}}]),
    );
    let failures = crate::audit::namespace::law::package_failures(
        &root,
        &json!({"resources":["runtime-product/flow.rs"]}),
    );
    assert!(
        contains(
            &failures,
            "namespace_mixed_domain_folder:runtime-product/flow.rs"
        ),
        "{failures:?}"
    );
    assert!(
        !contains(&failures, "namespace_law_missing_standards_row")
            && !contains(&failures, "namespace_law_missing_foundational_trace")
            && !contains(&failures, "namespace_law_present_only_as_prose"),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup namespace mixed domain");
}
