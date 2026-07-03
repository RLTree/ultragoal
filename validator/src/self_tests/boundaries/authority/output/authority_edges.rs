#[test]
fn claim_output_authority_rejects_absolute_private_path_writes() {
    let failures = crate::audit::law::authority_surfaces::output_authority_failures_for_test(
        "validator/src/cli/current_state/mod.rs",
        "let receipt_path = std::path::PathBuf::from(\"/tmp/claim-output.json\");\ncrate::json_boundary::write_json(&receipt_path, &value)?;",
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("var=receipt_path")),
        "private absolute claim output paths must not bypass typed output authority: {failures:?}"
    );
}

#[test]
fn claim_output_authority_ignores_fixture_module_materialization() {
    let failures = crate::audit::law::authority_surfaces::output_authority_failures_for_test(
        "validator/src/cli/current_state_tests.rs",
        "let receipt_path = root.join(\"validation_artifacts/current-state.json\");\ncrate::json_boundary::write_json(&receipt_path, &value)?;",
    );
    assert!(
        failures.is_empty(),
        "Rust test fixture materialization is not production claim output authority: {failures:?}"
    );
}
