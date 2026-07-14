use super::*;
use serde_json::json;
use std::fs;
use std::path::Path;

#[test]
fn strict_coverage_contract_rejects_unknown_and_routine_only_authority() {
    let cases: [(&str, fn(&mut serde_json::Value)); 3] = [
        ("unknown-top-level", |receipt: &mut serde_json::Value| {
            receipt["unexpected_authority"] = json!(true)
        }),
        ("unknown-nested", |receipt: &mut serde_json::Value| {
            receipt["coverage"]["unexpected"] = json!(true)
        }),
        ("routine-only-field", |receipt: &mut serde_json::Value| {
            receipt["coverage_cache_class"] = json!("retained_artifact_verified_local")
        }),
    ];
    for (label, mutate) in cases {
        let failures = strict_failures(label, mutate);
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("coverage_receipt_missing_or_malformed")),
            "{label}:{failures:?}"
        );
    }
}

#[test]
fn coverage_manifest_contract_rejects_unknown_authority() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "coverage-manifest-unknown-contract",
    );
    super::write_coverage_root(&root, 100.0, json!([]));
    let path = root.join(".harness/coverage-manifest.json");
    let mut manifest = crate::json_boundary::read_json(&path).expect("manifest");
    manifest["unexpected_authority"] = json!(true);
    crate::json_boundary::write_json(&path, &manifest).expect("mutated manifest");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let failures = super::super::super::validation::failures(
        &root,
        Path::new(COVERAGE_RECEIPT_REL),
        &candidate,
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("coverage_manifest_missing_or_malformed")),
        "{failures:?}"
    );
    fs::remove_dir_all(root).expect("cleanup manifest contract");
}

fn strict_failures(label: &str, mutate: impl FnOnce(&mut serde_json::Value)) -> Vec<String> {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(&format!(
        "coverage-receipt-contract-{label}"
    ));
    super::write_coverage_root(&root, 100.0, json!([]));
    let path = root.join(COVERAGE_RECEIPT_REL);
    let mut receipt = crate::json_boundary::read_json(&path).expect("receipt");
    mutate(&mut receipt);
    crate::json_boundary::write_json(&path, &receipt).expect("mutated receipt");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let failures = super::super::super::validation::failures(
        &root,
        Path::new(COVERAGE_RECEIPT_REL),
        &candidate,
    );
    fs::remove_dir_all(root).expect("cleanup receipt contract");
    failures
}
