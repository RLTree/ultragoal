use super::write_text;

#[test]
fn namespace_path_labels_classify_goal_work_and_generic_source_names() {
    for (path, expected) in [
        ("validator/src/audit/gate92/mod.rs", "gate_number"),
        ("validator/src/audit/gate/92/mod.rs", "gate_number"),
        ("validator/src/audit/phase4.rs", "phase_number"),
        ("validator/src/audit/phase/4/mod.rs", "phase_number"),
        (
            "validator/src/cli/observe/production-evidence/mod.rs",
            "production_evidence",
        ),
        ("validator/src/cli/observe/proofstatus.rs", "proof_status"),
    ] {
        assert_eq!(
            crate::audit::namespace::source::path_labels::product_opaque_goal_work_label(path),
            Some(expected),
            "{path}"
        );
    }

    for (leaf, expected) in [
        ("support", "support"),
        ("helper", "helper"),
        ("helpers", "helper"),
        ("utils", "utils"),
        ("utility", "utils"),
        ("utilities", "utils"),
        ("common", "common"),
        ("shared", "shared"),
        ("misc", "misc"),
        ("nodes", "nodes"),
    ] {
        assert_eq!(
            crate::audit::namespace::source::path_labels::generic_source_leaf_label(leaf),
            Some(expected),
            "{leaf}"
        );
    }

    for (identifier, expected) in [("shared", "shared"), ("misc", "misc")] {
        assert_eq!(
            crate::audit::namespace::source::path_labels::generic_identifier_bucket_label(
                identifier
            ),
            Some(expected),
            "{identifier}"
        );
    }

    assert_eq!(
        crate::audit::namespace::source::path_labels::product_opaque_goal_work_string_label(
            "fit_goal"
        ),
        Some("fit_goal")
    );
    for prose_or_api_string in [
        "serde_json::from_slice",
        "phase advancement",
        "progress claim only",
        "run-fit",
        "corr-fit",
        "sha256:fit",
        "validation_artifacts/observability/command-roundtrip/source-audit.json",
    ] {
        assert_eq!(
            crate::audit::namespace::source::path_labels::product_opaque_goal_work_string_label(
                prose_or_api_string
            ),
            None,
            "{prose_or_api_string}"
        );
    }
    for (session_label, expected) in [
        ("worker thread:", "session_history_worker_thread"),
        ("thread id:", "session_history_thread_id"),
        ("current phase:", "session_history_current_phase"),
        ("phase progress:", "session_history_phase_progress"),
        ("backlog item:", "session_history_backlog_item"),
        ("receipt status:", "session_history_receipt_status"),
        ("completion claim:", "session_history_completion_claim"),
    ] {
        assert_eq!(
            crate::audit::namespace::source::path_labels::product_opaque_goal_work_string_label(
                session_label
            ),
            Some(expected),
            "{session_label}"
        );
    }
    for (artifact_path, expected) in [
        (
            "validation_artifacts/observability/gate92/package-digest.json",
            "gate_number",
        ),
        (
            "validation_artifacts/observability/phase4/source-audit.json",
            "phase_number",
        ),
        (
            "validation_artifacts/observability/slice/source-audit.json",
            "slice",
        ),
        (
            "validation_artifacts/observability/progress/checkpoint.json",
            "progress",
        ),
        (
            "validation_artifacts/observability/proof-status/source-audit.json",
            "proof_status",
        ),
    ] {
        assert_eq!(
            crate::audit::namespace::source::path_labels::product_opaque_goal_work_string_label(
                artifact_path
            ),
            Some(expected),
            "{artifact_path}"
        );
    }
}

#[test]
fn namespace_identifier_scanner_handles_missing_files_and_incomplete_parameters() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("namespace-identifier-edges");
    let missing = crate::audit::namespace::source::identifiers::failures(
        &root,
        &["validator/src/missing.rs".to_string()],
    );
    assert!(missing.is_empty(), "{missing:?}");

    let rel = "validator/src/cli/observe/command_roundtrip/mod.rs";
    let numbered_stage_variant = format!("{}4", "Phase");
    write_text(
        &root.join(rel),
        &format!(
            r#"pub(crate) fn command_roundtrip(open: bool
pub(crate) fn
extern fn telemetry_reconciliation() {{}}
pub(crate) fn command_inventory(&self, _: &str, command_inventory: &str) {{}}
pub(crate) const COMMAND_ROUNDTRIP_RECEIPT: &str = "validation_artifacts/observability/command\"roundtrip.json";
enum CommandSurface {{
    // comment
    #[serde(rename = "command_roundtrip")]
    impl NotAVariant
    command_roundtrip,
    CommandRoundtrip,
    {numbered_stage_variant},
}}
"#
        ),
    );
    let failures =
        crate::audit::namespace::source::identifiers::failures(&root, &[rel.to_string()]);
    let failure_text = failures.join("\n");
    assert!(
        !failure_text.contains("identifier=command_roundtrip"),
        "{failures:?}"
    );
    assert!(!failure_text.contains("label=fit"), "{failures:?}");
    assert!(
        failure_text.contains("identifier=Phase4;label=phase_number"),
        "{failures:?}"
    );
    write_text(
        &root.join(rel),
        "pub(crate) fn command_roundtrip\npub(crate) fn telemetry_reconciliation() {}\n",
    );
    let no_open = crate::audit::namespace::source::identifiers::failures(&root, &[rel.to_string()]);
    let no_open_text = no_open.join("\n");
    assert!(
        !no_open_text.contains("namespace_validator_source_product_opaque"),
        "{no_open:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup namespace identifier edges");
}

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
