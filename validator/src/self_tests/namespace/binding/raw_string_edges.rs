use super::write_text;

#[test]
fn namespace_identifier_scanner_rejects_raw_string_goal_work_artifact_segments() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("namespace-raw-string");
    let rel = "validator/src/cli/observe/command_roundtrip/mod.rs";
    write_text(
        &root.join(rel),
        r##"pub(crate) const COMMAND_DIAGNOSTIC_RECEIPT: &str = r#"
{
  "receipt": "validation_artifacts/observability/fitting/source-audit.json"
}
"#;
"##,
    );

    let failures =
        crate::audit::namespace::source::identifiers::failures(&root, &[rel.to_string()]);
    assert!(
        failures.iter().any(|item| item.contains("label=fitting")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup namespace raw string");
}

#[test]
fn namespace_raw_string_scanner_keeps_negative_fixture_boundary_narrow() {
    let source = r##"pub(crate) const NEGATIVE_EXAMPLES: &str = r#"
{
  "bad_path": "validator/src/cli/observe/fitting/mod.rs"
}
"#;
"##;

    let failures = crate::audit::namespace::source::string_labels::raw_source_failures(
        "validator/src/self_tests/boundaries/authority/inventory/labels.rs",
        source,
    );
    assert!(
        failures.is_empty(),
        "negative fixture literals should stay isolated to their owner: {failures:?}"
    );
}

#[test]
fn namespace_raw_string_scanner_flags_dirty_source_before_parse_success() {
    let source = r##"pub(crate) const DIRTY_RECEIPT: &str =
    r#"validation_artifacts/observability/fitting/source-audit.json;
"##;

    let failures = crate::audit::namespace::source::string_labels::raw_source_failures(
        "validator/src/cli/observe/command_roundtrip/mod.rs",
        source,
    );
    assert!(
        failures.iter().any(|item| item.contains("label=fitting")),
        "{failures:?}"
    );
}

#[test]
fn namespace_raw_string_scanner_does_not_exempt_namespace_implementation() {
    let source = r##"pub(crate) const DIRTY_NAMESPACE_RECEIPT: &str =
    r#"validation_artifacts/observability/production-proof/source-audit.json"#;
"##;

    let failures = crate::audit::namespace::source::string_labels::raw_source_failures(
        "validator/src/audit/namespace/unrouted_authority.rs",
        source,
    );
    assert!(
        failures
            .iter()
            .any(|item| item.contains("label=production_proof")),
        "{failures:?}"
    );
}

#[test]
fn namespace_raw_string_scanner_does_not_exempt_arbitrary_namespace_self_test() {
    let source = r##"pub(crate) const DIRTY_TEST_RECEIPT: &str =
    r#"validation_artifacts/observability/fitting/source-audit.json"#;
"##;

    let failures = crate::audit::namespace::source::string_labels::raw_source_failures(
        "validator/src/self_tests/namespace/root_route_product_review.rs",
        source,
    );
    assert!(
        failures.iter().any(|item| item.contains("label=fitting")),
        "{failures:?}"
    );
}

#[test]
fn namespace_string_scanner_keeps_detector_catalog_allowance_narrow() {
    for label in ["workstream", "wip"] {
        let line = format!(
            "pub(crate) const LABEL: &str = \"validation_artifacts/observability/{label}/source-audit.json\";"
        );
        let detector = crate::audit::namespace::source::string_labels::failure(
            "validator/src/audit/namespace/source/path_labels.rs",
            1,
            &line,
        );
        assert!(
            detector.is_none(),
            "detector catalog must own rejection labels: {detector:?}"
        );

        let production = crate::audit::namespace::source::string_labels::failure(
            "validator/src/audit/namespace/source/unrouted_labels.rs",
            1,
            &line,
        )
        .expect("same label must fail outside detector catalog ownership");
        assert!(
            production.contains(&format!("label={label}")),
            "{production}"
        );
    }
}

#[test]
fn namespace_string_scanner_keeps_negative_fixture_allowance_narrow() {
    let line = "pub(crate) const BAD_HELPER: &str = \"todo_repair\";";
    let fixture = crate::audit::namespace::source::string_labels::failure(
        "validator/src/self_tests/namespace/binding/semantic_names.rs",
        1,
        line,
    );
    assert!(
        fixture.is_none(),
        "negative fixture owner must be able to name todo_repair: {fixture:?}"
    );

    let production = crate::audit::namespace::source::string_labels::failure(
        "validator/src/self_tests/namespace/unrouted_semantic_names.rs",
        1,
        line,
    )
    .expect("same label must fail outside negative fixture ownership");
    assert!(production.contains("label=todo_repair"), "{production}");
}
