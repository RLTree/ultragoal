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
        "current phase:",
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
