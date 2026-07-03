use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    crate::json_boundary::write_json(path, value).expect("write json");
}

fn harness(root: &Path) -> std::path::PathBuf {
    root.join(super::SOURCE_DIR)
}

#[test]
fn product_receipt_refresh_rejects_unsafe_artifact_paths() {
    let root = crate::self_tests::boundaries::support::temp_root("product-refresh-path");
    std::fs::create_dir_all(&root).expect("root");
    let mut value = json!({"path":"../escape.txt","digest":crate::digest::ZERO});
    let error = super::refresh_artifact_refs(&root, &mut value).expect_err("unsafe path");
    assert!(error.contains("invalid artifact path"), "{error}");
    std::fs::remove_dir_all(root).expect("cleanup product refresh path");
}

#[test]
fn product_journey_receipt_requires_fit_evidence_slot() {
    let root = crate::self_tests::boundaries::support::temp_root("product-journey-missing-fit");
    let dir = harness(&root);
    std::fs::create_dir_all(&dir).expect("harness");
    std::fs::write(dir.join("error.txt"), b"error evidence").expect("error evidence");
    write_json(
        &dir.join(super::JOURNEY),
        &json!({
            "schema":"harness-ultragoal.plugin-product-journey-receipt.v1",
            "status":"pass",
            "target_revision":{"kind":"package_digest","value":"old"},
            "generated_at":"2026-06-28T00:00:00Z",
            "claim_ceiling":"package_static_fixture_only",
            "journey":["a","b","c","d","e","f","g","h","i","j"],
            "evidence":[],
            "error_path_evidence":{"path":"validation_artifacts/harness/error.txt","digest":crate::digest::ZERO}
        }),
    );
    let error = super::journey_receipt(
        &root,
        Path::new("validation_artifacts/harness"),
        &crate::self_tests::boundaries::support::sha('a'),
        "2026-06-28T00:00:00Z",
    )
    .expect_err("missing fit evidence");
    assert!(error.contains("missing_fit_evidence"), "{error}");
    std::fs::remove_dir_all(root).expect("cleanup journey missing fit");
}

#[test]
fn product_journey_receipt_requires_current_fit_digest() {
    let root = crate::self_tests::boundaries::support::temp_root("product-journey-fit-digest");
    let dir = harness(&root);
    std::fs::create_dir_all(&dir).expect("harness");
    std::fs::write(dir.join("error.txt"), b"error evidence").expect("error evidence");
    write_json(
        &dir.join(super::JOURNEY),
        &json!({
            "schema":"harness-ultragoal.plugin-product-journey-receipt.v1",
            "status":"pass",
            "target_revision":{"kind":"package_digest","value":"old"},
            "generated_at":"2026-06-28T00:00:00Z",
            "claim_ceiling":"package_static_fixture_only",
            "journey":["a","b","c","d","e","f","g","h","i","j"],
            "evidence":[{"path":"validation_artifacts/harness/error.txt","digest":crate::digest::ZERO}],
            "error_path_evidence":{"path":"validation_artifacts/harness/error.txt","digest":crate::digest::ZERO}
        }),
    );
    let error = super::journey_receipt(
        &root,
        Path::new("validation_artifacts/harness"),
        &crate::self_tests::boundaries::support::sha('a'),
        "2026-06-28T00:00:00Z",
    )
    .expect_err("missing fit receipt");
    assert!(error.contains("fit-repo-receipt.json"), "{error}");
    std::fs::remove_dir_all(root).expect("cleanup journey fit digest");
}

#[test]
fn fit_receipt_requires_typed_plugin_version() {
    let root = crate::self_tests::boundaries::support::temp_root("product-fit-version");
    write_json(&root.join(".codex-plugin/plugin.json"), &json!({}));
    write_json(
        &harness(&root).join(super::FIT),
        &json!({
            "schema":"harness-ultragoal.fit-repo-receipt.v1",
            "status":"pass",
            "target_revision":{"kind":"package_digest","value":"old"},
            "claim_ceiling":"fit_repo_source_candidate_only",
            "receipt_digest":crate::digest::ZERO
        }),
    );
    let error = super::fit_receipt(
        &root,
        &crate::self_tests::boundaries::support::sha('b'),
        "2026-06-28T00:00:00Z",
    )
    .expect_err("missing plugin version");
    assert!(error.contains("missing version"), "{error}");
    std::fs::remove_dir_all(root).expect("cleanup product fit version");
}

#[test]
fn product_receipt_report_is_fail_closed_when_receipts_are_invalid_or_missing() {
    let root = crate::self_tests::boundaries::support::temp_root("product-report-fail");
    std::fs::create_dir_all(&root).expect("root");
    let rel = Path::new("validation_artifacts/harness");
    let report = super::report(
        &root,
        rel,
        &crate::self_tests::boundaries::support::sha('c'),
        &json!({}),
        &json!({}),
        &json!({}),
    );
    assert_eq!(report["status"], "fail");
    let failures = report["failures"].as_array().expect("failures");
    assert!(
        failures
            .iter()
            .any(|item| item.as_str().unwrap_or("").starts_with("fit_repo:"))
    );
    assert!(
        failures
            .iter()
            .any(|item| item.as_str().unwrap_or("").starts_with("product_fitness:"))
    );
    assert!(
        failures
            .iter()
            .any(|item| item.as_str().unwrap_or("").starts_with("product_journey:"))
    );
    for row in report["receipts"].as_array().expect("receipt rows") {
        assert_eq!(row["digest"], crate::digest::ZERO);
        assert_eq!(row["status"], "fail");
    }
    std::fs::remove_dir_all(root).expect("cleanup product report fail");
}

#[test]
fn product_receipt_minting_requires_package_digest_authority() {
    let root = crate::self_tests::boundaries::support::temp_root("product-mint-no-manifest");
    let out = root.join("receipts");
    std::fs::create_dir_all(&out).expect("out");
    let error = super::mint_all(&root, Path::new("receipts"), &out).expect_err("no package digest");
    assert!(error.contains("plugin-manifest-draft.json"), "{error}");
    std::fs::remove_dir_all(root).expect("cleanup product mint no manifest");
}
