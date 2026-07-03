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

#[test]
fn authority_surface_generated_artifacts_require_row_provenance_and_reject_hand_edits() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("generated-row-provenance");
    let rel = "docs/generated/observability/command-inventory.json";
    std::fs::create_dir_all(root.join("docs/generated/observability")).expect("generated dir");
    std::fs::write(
        root.join(rel),
        r#"{
  "schema":"harness-ultragoal.observability-command-inventory.v3",
  "generated_from":"test generator",
  "command_observability_inventory":{
    "package digest":{"observability_status":"observable"}
  }
}"#,
    )
    .expect("generated inventory without row provenance");
    let inventory = std::iter::once(rel.to_string()).collect();
    let failures = crate::audit::law::authority_surfaces::generated_boundary_failures_for_test(
        &root, &inventory,
    );
    assert!(
        failures.iter().any(|(_, failure)| failure.contains(
            "generated_inventory_row_missing_provenance:docs/generated/observability/command-inventory.json:command_observability_inventory:package digest"
        )),
        "{failures:?}"
    );

    std::fs::write(
        root.join(rel),
        r#"{
  "schema":"harness-ultragoal.observability-command-inventory.v3",
  "generated_from":"test generator",
  "manual_edits_allowed":true,
  "command_observability_inventory":{
    "package digest":{
      "current_owner_surface":"command:package digest",
      "validator_check_id":"package-digest-observability-binding",
      "hand_edited":true
    }
  }
}"#,
    )
    .expect("generated inventory with hand edit");
    let failures = crate::audit::law::authority_surfaces::generated_boundary_failures_for_test(
        &root, &inventory,
    );
    assert!(
        failures
            .iter()
            .any(|(_, failure)| failure.contains("generated_artifact_hand_edit_allowed")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|(_, failure)| failure.contains("generated_inventory_row_hand_edit_allowed")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup generated row provenance");
}

#[test]
fn authority_surface_generated_artifacts_reject_runtime_normalized_fixture_truth() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("runtime-normalized-fixture");
    let rel = "docs/generated/observability/command-inventory.json";
    std::fs::create_dir_all(root.join("docs/generated/observability")).expect("generated dir");
    std::fs::write(
        root.join(rel),
        r#"{
  "schema":"harness-ultragoal.observability-command-inventory.v3",
  "generated_from":"test generator",
  "runtime_normalized_fixture":{"artifact_truth":true},
  "command_observability_inventory":{
    "package digest":{
      "current_owner_surface":"command:package digest",
      "validator_check_id":"package-digest-observability-binding"
    }
  }
}"#,
    )
    .expect("runtime normalized fixture truth");
    let inventory = std::iter::once(rel.to_string()).collect();
    let failures = crate::audit::law::authority_surfaces::generated_boundary_failures_for_test(
        &root, &inventory,
    );
    assert!(
        failures
            .iter()
            .any(|(_, failure)| failure
                .contains("runtime_normalized_fixture_used_as_artifact_truth")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup runtime normalized fixture truth");
}
