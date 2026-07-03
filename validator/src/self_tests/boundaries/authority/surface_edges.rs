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
        "use serde_json::Value;\npub fn parse(root: &std::path::Path) -> Result<Value, String> { crate::json_boundary::read_json(root) }\n",
    );
    assert!(
        parser_boundary.is_empty(),
        "CLI parser boundary may consume raw JSON before emitting typed failures: {parser_boundary:?}"
    );

    let broad_cli_not_enough =
        crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
            "validator/src/cli/current_state/mod.rs",
            "use serde_json::Value;\npub fn pass_through(v: Value) -> Value { v }\n",
        );
    assert!(
        broad_cli_not_enough
            .iter()
            .any(|failure| failure.contains("raw_authority=raw_json")),
        "a broad CLI path prefix must not bless downstream raw JSON authority: {broad_cli_not_enough:?}"
    );

    let raw_field_access_not_parser =
        crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
            "validator/src/domain/claim_core.rs",
            "use serde_json::Value;\npub fn decide(v: Value) -> Value { v.get(\"status\").cloned().unwrap_or(Value::Null) }\n",
        );
    assert!(
        raw_field_access_not_parser
            .iter()
            .any(|failure| failure.contains("raw_authority=raw_json")),
        "raw JSON field access alone must not classify downstream law authority as a parser boundary: {raw_field_access_not_parser:?}"
    );

    let typed_failure_parser =
        crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
            "validator/src/audit/domain/claim_core.rs",
            "use serde_json::Value;\npub fn failures(v: &Value) -> Vec<String> { let mut out = Vec::new(); if v.get(\"status\").and_then(Value::as_str).is_none() { out.push(\"missing_status\".to_string()); } out }\n",
        );
    assert!(
        typed_failure_parser.is_empty(),
        "raw receipt fields may be consumed only when converted into typed failures before law execution: {typed_failure_parser:?}"
    );

    let typed_record_projection =
        crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
            "validator/src/audit/domain/claim_projection.rs",
            "use serde_json::Value;\npub struct ClaimState { pub status: String }\npub fn project(v: &Value) -> ClaimState { ClaimState { status: v.get(\"status\").and_then(Value::as_str).unwrap_or(\"\").to_string() } }\n",
        );
    assert!(
        typed_record_projection.is_empty(),
        "raw JSON fields may feed typed record projections before downstream law execution: {typed_record_projection:?}"
    );

    let result_value_passthrough =
        crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
            "validator/src/domain/claim_core.rs",
            "use serde_json::Value;\npub fn wrap(v: Value) -> Result<Value, String> { Ok(v) }\n",
        );
    assert!(
        result_value_passthrough
            .iter()
            .any(|failure| failure.contains("raw_authority=raw_json")),
        "Result<Value, String> is still raw JSON authority unless it reads structured input or emits typed failures: {result_value_passthrough:?}"
    );

    for (label, text) in [
        (
            "raw_path",
            "pub fn decide(raw_path: &str) -> &str { raw_path }\n",
        ),
        (
            "raw_string",
            "pub fn decide(raw_string: String) -> String { raw_string }\n",
        ),
        (
            "raw_map",
            "use serde_json::Value;\nuse std::collections::BTreeMap;\npub fn decide(raw_map: BTreeMap<String, Value>) -> BTreeMap<String, Value> { raw_map }\n",
        ),
    ] {
        let failures = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
            "validator/src/domain/claim_core.rs",
            text,
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains(&format!("raw_authority={label}"))),
            "{label} should be rejected as unclassified downstream authority: {failures:?}"
        );
    }

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
#[test]
fn raw_authority_classification_is_not_blessed_by_unrelated_json_projection() {
    let failures = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/domain/claim_core.rs",
        "use serde_json::{Value, json};\npub fn decide(v: Value) -> Value { let _receipt = json!({\"status\":\"pass\"}); v }\n",
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("raw_downstream_authority_unclassified")),
        "json projection text elsewhere in a file must not classify raw downstream authority: {failures:?}"
    );
}

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
