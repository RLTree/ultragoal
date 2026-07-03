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
}
