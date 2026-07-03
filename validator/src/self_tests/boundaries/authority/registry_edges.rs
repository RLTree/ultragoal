use serde_json::json;
use std::collections::BTreeSet;

#[test]
fn mandatory_law_registry_requires_each_foundational_authority_law() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "authority-registry-missing-law",
    );
    std::fs::create_dir_all(&root).expect("registry fixture root");
    let failures = crate::audit::law::authority_surfaces::registry_law_failures_for_test(
        &root,
        &json!({"laws":[]}),
        &BTreeSet::new(),
        &BTreeSet::new(),
    );
    assert!(
        failures
            .iter()
            .any(|(_, failure)| failure.contains("authority_surface_law_missing")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup missing law registry fixture");
}

#[test]
fn mandatory_law_registry_rejects_miswired_checks_and_missing_fields() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "authority-registry-miswired-check",
    );
    std::fs::create_dir_all(&root).expect("registry fixture root");
    let failures = crate::audit::law::authority_surfaces::registry_law_failures_for_test(
        &root,
        &json!({"laws":[{
            "law_id":"authority-source-binding",
            "validator_check_id":"wrong-check",
            "law_specific":{}
        }]}),
        &BTreeSet::new(),
        &BTreeSet::new(),
    );
    assert!(
        failures.iter().any(|(_, failure)| {
            failure.contains("authority_surface_wrong_validator_check:authority-source-binding")
        }),
        "{failures:?}"
    );
    assert!(
        failures.iter().any(|(_, failure)| {
            failure.contains(
                "authority_surface_field_missing:authority-source-binding:valid_fixture_path",
            )
        }),
        "{failures:?}"
    );
    assert!(
        failures.iter().any(|(_, failure)| {
            failure.contains(
                "authority_surface_guard_missing:authority-source-binding:parser_boundary_projection_or_catalog_classification"
            )
        }),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|(_, failure)| failure.contains("authority_surface_red_fixture_missing")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup miswired registry fixture");
}

#[test]
fn mandatory_law_registry_rejects_unpackaged_fixture_paths() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "authority-registry-unpackaged",
    );
    std::fs::create_dir_all(&root).expect("registry fixture root");
    let red_id =
        "authority-source-binding-parser-boundary-projection-or-catalog-classification-red";
    let red_ids = std::iter::once(red_id.to_string()).collect::<BTreeSet<_>>();
    let failures = crate::audit::law::authority_surfaces::registry_law_failures_for_test(
        &root,
        &json!({"laws":[{
            "law_id":"authority-source-binding",
            "validator_check_id":"authority-source-binding",
            "valid_fixture_path":"fixtures/mandatory-law-surfaces/valid/authority-source-binding.json",
            "law_specific":{
                "parser_boundary_projection_or_catalog_classification":true,
                "typed_downstream_records":true,
                "stale_digest_rejected":true,
                "source_obligation_parity":true
            }
        }]}),
        &red_ids,
        &BTreeSet::new(),
    );
    assert!(
        failures.iter().any(|(_, failure)| {
            failure.contains(
                "authority_surface_path_not_packaged:authority-source-binding:fixtures/mandatory-law-surfaces/valid/authority-source-binding.json"
            )
        }),
        "{failures:?}"
    );
    assert!(
        failures.iter().any(|(_, failure)| {
            failure.contains(&format!(
                "authority_surface_red_fixture_not_packaged:{red_id}"
            ))
        }),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup unpackaged registry fixture");
}
