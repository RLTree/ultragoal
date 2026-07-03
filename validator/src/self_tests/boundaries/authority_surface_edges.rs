#[test]
fn authority_surface_source_scans_reject_raw_downstream_claims_and_claim_output_bypasses() {
    let raw_downstream = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/domain/claim_core.rs",
        "use serde_json::Value;\npub fn decide(v: Value) -> Value { v }\n",
    );
    assert!(
        raw_downstream
            .iter()
            .any(|failure| failure.contains("raw_downstream_authority_unclassified")),
        "{raw_downstream:?}"
    );

    let parser_boundary = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/cli/current_state/mod.rs",
        "use serde_json::Value;\npub fn parse(v: Value) -> Value { v }\n",
    );
    assert!(
        parser_boundary.is_empty(),
        "CLI parser boundary may consume raw JSON before emitting typed failures: {parser_boundary:?}"
    );

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

    let fixture_text = crate::audit::law::authority_surfaces::output_authority_failures_for_test(
        "validator/src/cli/current_state/tests.rs",
        "let path = root.join(&command.receipt);",
    );
    assert!(
        fixture_text.is_empty(),
        "test fixture materialization is validation code, not a production claim writer: {fixture_text:?}"
    );
}
