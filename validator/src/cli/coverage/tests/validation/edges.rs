use super::*;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

#[test]
fn coverage_validation_rejects_scalar_completion_and_missing_receipts() {
    let missing_root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("coverage-missing");
    let missing = validation_failures(
        &missing_root,
        Path::new(COVERAGE_RECEIPT_REL),
        crate::digest::ZERO,
    );
    assert!(has(&missing, "coverage_receipt_missing_or_malformed"));
    let _ = fs::remove_dir_all(missing_root);

    for (field, value, code) in [
        ("schema", json!("wrong"), "coverage_receipt_malformed"),
        ("command", json!(""), "coverage_command_missing"),
        ("tool", json!(""), "coverage_tool_missing"),
        (
            "coverage_target_dir",
            json!(""),
            "coverage_target_dir_missing",
        ),
        (
            "coverage_target_dir",
            json!("target"),
            "coverage_target_dir_not_isolated",
        ),
        (
            "tool_version",
            json!(""),
            "coverage_receipt_tool_version_missing",
        ),
        (
            "generated_by",
            json!("handwritten"),
            "coverage_receipt_not_tool_generated",
        ),
        (
            "percent_source",
            json!("prose"),
            "coverage_percent_from_prose",
        ),
        ("command_exit", json!(1), "coverage_command_failed"),
        (
            "workspace_root",
            json!("/elsewhere"),
            "coverage_receipt_workspace_mismatch",
        ),
        (
            "target_paths",
            json!([]),
            "coverage_claim_missing_target_paths",
        ),
        (
            "measured_dimensions",
            json!([]),
            "coverage_claim_missing_dimensions",
        ),
        (
            "claim_ceiling",
            json!("withheld_or_blocked"),
            "coverage_ratchet_presented_as_complete",
        ),
        (
            "supported_claim_classes",
            json!([]),
            "coverage_ratchet_presented_as_complete",
        ),
        (
            "blocked_claim_classes",
            json!([]),
            "coverage_blocked_claim_missing:completion",
        ),
        (
            "coverage",
            json!({"percent":100.0,"floor_percent":100,"policy":"ratchet_only"}),
            "coverage_ratchet_presented_as_complete",
        ),
    ] {
        let failures = mutated_failures(|receipt| receipt[field] = value.clone());
        assert!(has(&failures, code), "{field}: {failures:?}");
    }

    let target_kind = mutated_failures(|receipt| receipt["target_revision"]["kind"] = json!("git"));
    assert!(has(&target_kind, "coverage_receipt_target_kind_mismatch"));
    let target_value =
        mutated_failures(|receipt| receipt["target_revision"]["value"] = json!("bad"));
    assert!(has(
        &target_value,
        "coverage_receipt_target_digest_mismatch"
    ));
    let uncovered = mutated_failures(|receipt| {
        receipt["coverage"]["percent"] = json!(99.0);
        receipt["uncovered_records"] = json!([{"path":"src/lib.rs"}]);
    });
    assert!(has(&uncovered, "coverage_claim_uncovered_code"));
}
#[test]
fn coverage_validation_rejects_digest_report_and_manifest_edges() {
    for (field, value, code) in [
        (
            "coverage_manifest_digest",
            json!(crate::digest::ZERO),
            "coverage_receipt_manifest_digest_mismatch",
        ),
        (
            "coverage_command_digest",
            json!(crate::digest::ZERO),
            "coverage_receipt_command_digest_mismatch",
        ),
        (
            "source_tree_digest",
            json!(crate::digest::ZERO),
            "coverage_receipt_source_digest_mismatch",
        ),
        (
            "changed_files_digest",
            json!(crate::digest::ZERO),
            "coverage_receipt_changed_files_digest_mismatch",
        ),
        (
            "target_paths",
            json!(["other.rs"]),
            "coverage_manifest_target_gap",
        ),
        (
            "measured_dimensions",
            json!(["branch"]),
            "coverage_required_dimension_missing",
        ),
    ] {
        let failures = mutated_failures(|receipt| receipt[field] = value.clone());
        assert!(has(&failures, code), "{field}: {failures:?}");
    }

    let missing_report =
        mutated_failures(|receipt| receipt["machine_readable_report"]["path"] = json!(""));
    assert!(has(&missing_report, "coverage_report_missing"));
    let escaped_report = mutated_failures(|receipt| {
        receipt["machine_readable_report"]["path"] = json!("../escape.json")
    });
    assert!(has(&escaped_report, "coverage_report_missing"));
    let bad_json = mutated_root_failures(|root, receipt| {
        fs::write(
            root.join("validation_artifacts/coverage/llvm-cov-full.json"),
            "nope",
        )
        .expect("bad report");
        receipt["machine_readable_report"]["digest"] = json!(
            crate::digest::file(&root.join("validation_artifacts/coverage/llvm-cov-full.json"))
                .expect("report digest")
        );
    });
    assert!(has(&bad_json, "coverage_report_not_machine_readable"));
    let digest = mutated_failures(|receipt| {
        receipt["machine_readable_report"]["digest"] = json!(crate::digest::ZERO)
    });
    assert!(has(&digest, "coverage_report_digest_mismatch"));

    let missing_manifest = mutated_root_failures(|root, _receipt| {
        fs::remove_file(root.join(".harness/coverage-manifest.json")).expect("remove manifest");
    });
    assert!(has(
        &missing_manifest,
        "coverage_manifest_missing_or_malformed"
    ));

    let digest_error = mutated_root_failures(|root, _receipt| {
        let manifest = json!({
            "schema": "harness-ultragoal.coverage-manifest.v1",
            "required_target_paths": ["src/lib.rs"],
            "changed_file_coupling_policy": {
                "required": true,
                "changed_files": ["missing.rs"]
            },
            "required_measured_dimensions_per_root": [{
                "root": "src",
                "dimensions": ["line"]
            }],
            "source_discovery_rules": {"ignore": []},
            "exclusions": []
        });
        crate::json_boundary::write_json(&root.join(".harness/coverage-manifest.json"), &manifest)
            .expect("manifest with missing changed file");
    });
    assert!(has(
        &digest_error,
        "coverage_receipt_changed_files_digest_mismatch:"
    ));

    let source_digest_error = mutated_root_failures(|root, _receipt| {
        let manifest = json!({
            "schema": "harness-ultragoal.coverage-manifest.v1",
            "required_target_paths": ["../escape.rs"],
            "changed_file_coupling_policy": {
                "required": true,
                "changed_files": ["src/lib.rs"]
            },
            "required_measured_dimensions_per_root": [{
                "root": "src",
                "dimensions": ["line"]
            }],
            "source_discovery_rules": {"ignore": []},
            "exclusions": []
        });
        crate::json_boundary::write_json(&root.join(".harness/coverage-manifest.json"), &manifest)
            .expect("manifest with missing source file");
    });
    assert!(has(
        &source_digest_error,
        "coverage_receipt_source_digest_mismatch:"
    ));
}

fn mutated_failures(mut mutate: impl FnMut(&mut Value)) -> Vec<String> {
    mutated_root_failures(|_, receipt| mutate(receipt))
}

fn mutated_root_failures(mut mutate: impl FnMut(&Path, &mut Value)) -> Vec<String> {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("coverage-validation-edge");
    super::write_coverage_root(&root, 100.0, json!([]));
    let path = root.join(COVERAGE_RECEIPT_REL);
    let mut receipt = crate::json_boundary::read_json(&path).expect("receipt");
    mutate(&root, &mut receipt);
    crate::json_boundary::write_json(&path, &receipt).expect("write receipt");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let failures = validation_failures(&root, Path::new(COVERAGE_RECEIPT_REL), &candidate);
    fs::remove_dir_all(root).expect("cleanup validation edge");
    failures
}

fn has(failures: &[String], needle: &str) -> bool {
    failures.iter().any(|failure| failure.contains(needle))
}

fn validation_failures(root: &Path, receipt: &Path, candidate: &str) -> Vec<String> {
    super::super::super::validation::failures(root, receipt, candidate)
}
