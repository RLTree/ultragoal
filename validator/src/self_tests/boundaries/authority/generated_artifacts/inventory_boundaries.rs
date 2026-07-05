#[test]
fn generated_inventory_rejects_builder_contract_inputs_and_missing_provenance() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "generated-inventory-boundaries",
    );
    let rel = "docs/generated/observability/command-inventory.json";
    std::fs::create_dir_all(root.join("docs/generated/observability")).expect("generated dir");
    std::fs::write(root.join(rel), r#"{"command_observability_inventory":{}}"#)
        .expect("generated artifact without provenance");
    let inventory = [
        rel.to_string(),
        "docs/ultragoal-contract-2026-07/README.md".to_string(),
    ]
    .into_iter()
    .collect();
    let failures = crate::audit::law::authority_surfaces::generated_boundary_failures_for_test(
        &root, &inventory,
    );
    let failure_text = failure_text(&failures);
    assert!(
        failure_text.contains("builder_contract_file_in_package_evidence"),
        "{failures:?}"
    );
    assert!(
        failure_text.contains("generated_artifact_missing_provenance"),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup generated inventory boundaries");
}

#[test]
fn generated_inventory_rejects_unlisted_generated_artifacts() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("generated-unlisted-artifact");
    let rel = "docs/generated/observability/command-inventory.json";
    std::fs::create_dir_all(root.join("docs/generated/observability")).expect("generated dir");
    std::fs::write(root.join(rel), r#"{"generated_from":"spec"}"#)
        .expect("generated artifact with provenance");
    let failures = crate::audit::law::authority_surfaces::generated_boundary_failures_for_test(
        &root,
        &std::collections::BTreeSet::new(),
    );
    assert!(
        failures
            .iter()
            .any(|(_, failure)| failure.contains("generated_artifact_not_in_package_inventory")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup unlisted generated artifact");
}

#[test]
fn generated_inventory_scans_generated_examples_and_accepts_ready_receipt_provenance() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("generated-example-artifact");
    let rel = "examples/generated/READY_FOR_MERGE.example.json";
    std::fs::create_dir_all(root.join("examples/generated")).expect("generated examples dir");
    std::fs::write(
        root.join(rel),
        r#"{
  "schema":"harness-ultragoal.ready-for-merge.v1",
  "generated_by":"ultragoal-audit",
  "provenance":{
    "validator_receipt":{"path":"validation_artifacts/ultragoal-audit/validator-receipt.json","digest":"sha256:0000000000000000000000000000000000000000000000000000000000000000"},
    "generated_artifact_type":"ready_for_merge",
    "validator_run_id":"run-example",
    "input_manifest_digest":"sha256:0000000000000000000000000000000000000000000000000000000000000000"
  }
}"#,
    )
    .expect("ready receipt generated example");
    let inventory = std::iter::once(rel.to_string()).collect();
    let failures = crate::audit::law::authority_surfaces::generated_boundary_failures_for_test(
        &root, &inventory,
    );
    let failure_text = failure_text(&failures);
    assert!(
        !failure_text.contains("generated_artifact_missing_provenance"),
        "{failures:?}"
    );

    std::fs::write(root.join(rel), r#"{"schema":"example"}"#)
        .expect("generated example without provenance");
    let failures = crate::audit::law::authority_surfaces::generated_boundary_failures_for_test(
        &root, &inventory,
    );
    assert!(
        failures
            .iter()
            .any(|(_, failure)| failure.contains("generated_artifact_missing_provenance")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup generated example artifact");
}

#[test]
fn generated_inventory_rejects_non_object_generated_artifacts_without_provenance() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "generated-non-object-artifact",
    );
    let rel = "docs/generated/observability/command-inventory.json";
    std::fs::create_dir_all(root.join("docs/generated/observability")).expect("generated dir");
    std::fs::write(root.join(rel), "[]").expect("generated array artifact");
    let inventory = std::iter::once(rel.to_string()).collect();
    let failures = crate::audit::law::authority_surfaces::generated_boundary_failures_for_test(
        &root, &inventory,
    );
    assert!(
        failures
            .iter()
            .any(|(_, failure)| failure.contains("generated_artifact_missing_provenance")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup non-object generated artifact");
}

#[test]
fn generated_inventory_accepts_typed_provenance_sources() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("generated-typed-provenance");
    let rel = "docs/generated/observability/command-inventory.json";
    std::fs::create_dir_all(root.join("docs/generated/observability")).expect("generated dir");
    std::fs::write(
        root.join(rel),
        r#"{
  "provenance":{"generated_from":"command-observability-spec"},
  "command_observability_inventory":{
    "package digest":{
      "owner_surface":"command:package digest",
      "validator_check_id":"package-digest-observability-binding"
    },
    "source audit":{
      "owner_surface":"command:source audit",
      "source_spec":{"id":"source-audit-command"}
    }
  }
}"#,
    )
    .expect("generated artifact with typed provenance");
    let inventory = std::iter::once(rel.to_string()).collect();
    let failures = crate::audit::law::authority_surfaces::generated_boundary_failures_for_test(
        &root, &inventory,
    );
    let failure_text = failure_text(&failures);
    assert!(
        !failure_text.contains("generated_artifact_missing_provenance"),
        "{failures:?}"
    );
    assert!(
        !failure_text.contains("generated_inventory_row_missing_provenance"),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup typed generated provenance");
}

fn failure_text(failures: &[(String, String)]) -> String {
    let mut text = String::new();
    for (_, failure) in failures {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(failure);
    }
    text
}
