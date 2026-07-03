use super::*;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn coverage_validation_rejects_absolute_receipts_before_claim_authority() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "coverage-absolute-receipt-rejected",
    );
    super::write_coverage_root(&root, 100.0, json!([]));
    let saved = root.with_extension("coverage-receipt.json");
    fs::copy(root.join(COVERAGE_RECEIPT_REL), &saved).expect("save receipt");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");

    let failures = super::super::super::validation::failures(&root, Path::new(&saved), &candidate);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("coverage_receipt_path_invalid")),
        "{failures:?}"
    );
    fs::remove_dir_all(&root).expect("cleanup root");
    fs::remove_file(saved).expect("cleanup saved receipt");
}

#[test]
fn coverage_validation_accepts_relative_receipts_and_template_fallbacks() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("coverage-template-fallback");
    super::write_coverage_root(&root, 100.0, json!([]));
    fs::create_dir_all(root.join("templates/.harness")).expect("template dir");
    fs::rename(
        root.join(".harness/coverage-manifest.json"),
        root.join("templates/.harness/coverage-manifest.json"),
    )
    .expect("move manifest");
    fs::rename(
        root.join(".harness/coverage-command"),
        root.join("templates/.harness/coverage-command"),
    )
    .expect("move command");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let receipt = PathBuf::from(COVERAGE_RECEIPT_REL);
    let failures = super::super::super::validation::failures(&root, &receipt, &candidate);
    assert!(failures.is_empty(), "{failures:?}");
    fs::remove_dir_all(root).expect("cleanup template");
}
