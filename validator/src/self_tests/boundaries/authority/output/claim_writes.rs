#[test]
fn claim_artifact_writes_require_typed_output_authority() {
    let output_bypass = crate::audit::law::authority_surfaces::output_authority_failures_for_test(
        "validator/src/cli/current_state/mod.rs",
        "let path = root.join(&command.receipt);\ncrate::json_boundary::write_json(&path, &value)?;",
    );
    assert!(
        output_bypass
            .iter()
            .any(|failure| failure.contains("claim_artifact_output_without_typed_authority")),
        "{output_bypass:?}"
    );

    let typed_output = crate::audit::law::authority_surfaces::output_authority_failures_for_test(
        "validator/src/cli/current_state/mod.rs",
        "let path = crate::output_path::claim_artifact_path(root, &command.receipt, \"receipt\")?;",
    );
    assert!(
        typed_output.is_empty(),
        "typed claim-artifact authority should satisfy output path enforcement: {typed_output:?}"
    );

    let mixed_output = crate::audit::law::authority_surfaces::output_authority_failures_for_test(
        "validator/src/cli/current_state/mod.rs",
        "let safe_path = crate::output_path::claim_artifact_path(root, &command.receipt, \"receipt\")?;\nlet unsafe_path = root.join(&command.receipt);\ncrate::json_boundary::write_json(&unsafe_path, &value)?;",
    );
    assert!(
        mixed_output
            .iter()
            .any(|failure| failure.contains("claim_artifact_output_without_typed_authority")),
        "one typed writer must not bless a sibling raw claim output path: {mixed_output:?}"
    );

    let renamed_bypass = crate::audit::law::authority_surfaces::output_authority_failures_for_test(
        "validator/src/cli/live_loop/mod.rs",
        "let current_state_path = root.join(\"validation_artifacts/current-state.json\");\ncrate::json_boundary::write_json(&current_state_path, &value)?;",
    );
    assert!(
        renamed_bypass
            .iter()
            .any(|failure| failure.contains("var=current_state_path")),
        "raw root-relative claim output writes must fail even when the variable name is not receipt-shaped: {renamed_bypass:?}"
    );

    let directory_join_bypass =
        crate::audit::law::authority_surfaces::output_authority_failures_for_test(
            "validator/src/cli/product/receipts/mod.rs",
            "let product_path = out_dir.join(\"product-fitness-receipt.json\");\ncrate::json_boundary::write_json(&product_path, &value)?;",
        );
    assert!(
        directory_join_bypass
            .iter()
            .any(|failure| failure.contains("var=product_path")),
        "claim receipts written under a caller-controlled directory must use typed output authority: {directory_join_bypass:?}"
    );

    let local_claim_path_resolver_bypass =
        crate::audit::law::authority_surfaces::output_authority_failures_for_test(
            "validator/src/cli/routine.rs",
            "let out = output_path(root, &command.receipt)?;\ncrate::json_boundary::write_json(&out, &value)?;\nfn output_path(root: &Path, path: &Path) -> Result<PathBuf, String> { Ok(root.join(path)) }\n",
        );
    assert!(
        local_claim_path_resolver_bypass
            .iter()
            .any(|failure| failure.contains("var=out")),
        "local output_path functions must not create parallel claim-output authority: {local_claim_path_resolver_bypass:?}"
    );

    let direct_join_bypass =
        crate::audit::law::authority_surfaces::output_authority_failures_for_test(
            "validator/src/cli/current_state/mod.rs",
            "crate::json_boundary::write_json(&root.join(\"validation_artifacts/current-state.json\"), &value)?;",
        );
    assert!(
        direct_join_bypass
            .iter()
            .any(|failure| failure.contains("claim_artifact_output_without_typed_authority")),
        "direct root.join claim writes must route through output_path::claim_artifact_path: {direct_join_bypass:?}"
    );

    let direct_fs_write_bypass =
        crate::audit::law::authority_surfaces::output_authority_failures_for_test(
            "validator/src/cli/current_state/mod.rs",
            "std::fs::write(root.join(\"validation_artifacts/current-state.json\"), payload)?;",
        );
    assert!(
        direct_fs_write_bypass
            .iter()
            .any(|failure| failure.contains("claim_artifact_output_without_typed_authority")),
        "direct filesystem writes to claim artifacts must route through typed output authority: {direct_fs_write_bypass:?}"
    );

    let fs_write_var_bypass =
        crate::audit::law::authority_surfaces::output_authority_failures_for_test(
            "validator/src/cli/current_state/mod.rs",
            "let path = root.join(\"validation_artifacts/current-state.json\");\nstd::fs::write(&path, payload)?;",
        );
    assert!(
        fs_write_var_bypass
            .iter()
            .any(|failure| failure.contains("var=path")),
        "filesystem writes through raw output variables must fail closed: {fs_write_var_bypass:?}"
    );

    let typed_var_write = crate::audit::law::authority_surfaces::output_authority_failures_for_test(
        "validator/src/cli/live_loop/mod.rs",
        "let current_state_path = crate::output_path::literal_claim_artifact_path(root, \"validation_artifacts/current-state.json\", \"current state snapshot\");\ncrate::json_boundary::write_json(&current_state_path, &value)?;",
    );
    assert!(
        typed_var_write.is_empty(),
        "typed output-authority variables may be written: {typed_var_write:?}"
    );

    let scanner_catalog = crate::audit::law::authority_surfaces::output_authority_failures_for_test(
        "validator/src/audit/law/authority_surfaces/source/output.rs",
        "    \"root.join(&command.receipt)\",\n",
    );
    assert!(
        scanner_catalog.is_empty(),
        "the scanner pattern catalog is not a claim writer: {scanner_catalog:?}"
    );

    let fixture_text = crate::audit::law::authority_surfaces::output_authority_failures_for_test(
        "validator/src/cli/current_state/tests.rs",
        "let path = root.join(&command.receipt);",
    );
    assert!(
        fixture_text.is_empty(),
        "test fixture materialization is validation code, not a production claim writer: {fixture_text:?}"
    );

    let absolute_output = crate::audit::law::authority_surfaces::output_authority_failures_for_test(
        "validator/src/cli/current_state/mod.rs",
        "let path = PathBuf::from(&command.receipt);\ncrate::json_boundary::write_json(&path, &value)?;",
    );
    assert!(
        absolute_output
            .iter()
            .any(|failure| failure.contains("claim_artifact_output_without_typed_authority")),
        "absolute or caller-controlled claim outputs must route through typed output authority: {absolute_output:?}"
    );

    let constant_receipt_join =
        crate::audit::law::authority_surfaces::output_authority_failures_for_test(
            "validator/src/cli/package/digest.rs",
            "crate::json_boundary::write_json(&root.join(RECEIPT_REL), &value)?;",
        );
    assert!(
        constant_receipt_join
            .iter()
            .any(|failure| failure.contains("claim_artifact_output_without_typed_authority")),
        "constant receipt paths still need typed output authority: {constant_receipt_join:?}"
    );
}
