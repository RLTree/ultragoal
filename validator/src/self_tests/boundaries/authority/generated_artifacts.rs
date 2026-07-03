#[test]
fn generated_artifact_inventory_rows_require_provenance_and_reject_hand_edits() {
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
  "surface_inventory":{
    "validator check families":{"observability_status":"observable"}
  },
  "new_product_inventory":{
    "future family":{"observability_status":"observable"}
  }
}"#,
    )
    .expect("generated inventory family without row provenance");
    let failures = crate::audit::law::authority_surfaces::generated_boundary_failures_for_test(
        &root, &inventory,
    );
    assert!(
        failures.iter().any(|(_, failure)| failure.contains(
            "generated_inventory_row_missing_provenance:docs/generated/observability/command-inventory.json:surface_inventory:validator check families"
        )),
        "{failures:?}"
    );
    assert!(
        failures.iter().any(|(_, failure)| failure.contains(
            "generated_inventory_row_missing_provenance:docs/generated/observability/command-inventory.json:new_product_inventory:future family"
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
fn generated_artifacts_reject_product_opaque_artifact_path_segments() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("generated-path-segment");
    let rel = "docs/generated/observability/command-inventory.json";
    std::fs::create_dir_all(root.join("docs/generated/observability")).expect("generated dir");
    let product_opaque_root = ascii(&[102, 105, 116]);
    let process_noun = format!("{product_opaque_root}ting");
    let evidence_noun = [
        ascii(&[112, 114, 111, 100, 117, 99, 116, 105, 111, 110]),
        "-".to_string(),
        ascii(&[112, 114, 111, 111, 102]),
    ]
    .concat();
    let process_label = process_noun.clone();
    let evidence_label = [
        ascii(&[112, 114, 111, 100, 117, 99, 116, 105, 111, 110]),
        "_".to_string(),
        ascii(&[112, 114, 111, 111, 102]),
    ]
    .concat();
    let process_path =
        format!("validation_artifacts/observability/{process_noun}/package-digest.json");
    let evidence_path =
        format!("validation_artifacts/observability/{evidence_noun}/source-audit.json");
    std::fs::write(
        root.join(rel),
        format!(
            r#"{{
  "schema":"harness-ultragoal.observability-command-inventory.v3",
  "generated_from":"test generator",
  "command_observability_inventory":{{
    "package digest":{{
      "current_owner_surface":"command:package digest",
      "validator_check_id":"package-digest-observability-binding",
      "receipt_paths":[
        "{process_path}",
        "{evidence_path}"
      ]
    }}
  }}
}}"#
        ),
    )
    .expect("generated artifact path segment");
    let inventory = std::iter::once(rel.to_string()).collect();
    let failures = crate::audit::law::authority_surfaces::generated_boundary_failures_for_test(
        &root, &inventory,
    );
    assert!(
        failures.iter().any(|(_, failure)| failure
            .contains("generated_artifact_product_opaque_path_segment")
            && failure.contains(&format!("label={process_label}"))),
        "{failures:?}"
    );
    assert!(
        failures.iter().any(|(_, failure)| failure
            .contains("generated_artifact_product_opaque_path_segment")
            && failure.contains(&format!("label={evidence_label}"))),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup generated path segment");
}

#[test]
fn generated_artifacts_reject_runtime_normalized_fixture_truth() {
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

fn ascii(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("ascii fixture token")
}
