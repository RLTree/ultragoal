#[test]
fn output_authority_rejects_observability_and_roundtrip_receipt_bypasses() {
    for (label, source) in [
        (
            "observability_receipt_join",
            "let path = root.join(&observability_receipt);\ncrate::json_boundary::write_json(&path, &value)?;",
        ),
        (
            "command_observability_receipt_join",
            "let path = root.join(&command.observability_receipt);\ncrate::json_boundary::write_json(&path, &value)?;",
        ),
        (
            "command_receipt_rel_join",
            "crate::json_boundary::write_json(&root.join(command.receipt_rel()), &value)?;",
        ),
        (
            "direct_receipt_write",
            "crate::json_boundary::write_json(receipt, &value)?;",
        ),
        (
            "gc_observability_receipt_join",
            "crate::json_boundary::write_json(&root.join(observability_path), &obs)?;",
        ),
        (
            "registry_active_receipt_join",
            "crate::json_boundary::write_json(&root.join(ACTIVE_RECEIPT), &receipt)?;",
        ),
    ] {
        let failures = crate::audit::law::authority_surfaces::output_authority_failures_for_test(
            "validator/src/cli/observe/command_roundtrip/mod.rs",
            source,
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("claim_artifact_output_without_typed_authority")),
            "{label} should be rejected as claim output without typed authority: {failures:?}"
        );
    }
}
