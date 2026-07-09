use super::*;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

#[test]
fn routine_coverage_accepts_verified_lineage_and_rejects_claim_overreach() {
    let pass = routine_failures(|_root, _receipt| {});
    assert!(pass.is_empty(), "{pass:?}");

    for (field, value, code) in [
        (
            "claim_ceiling",
            json!("supports_complete_coverage_claim"),
            "coverage_routine_claim_ceiling_not_routine_only",
        ),
        (
            "coverage_cache_class",
            json!(""),
            "coverage_routine_cache_class_missing",
        ),
        (
            "cargo_version",
            json!(""),
            "coverage_routine_cargo_version_missing",
        ),
        (
            "rustc_version",
            json!(""),
            "coverage_routine_rustc_version_missing",
        ),
        (
            "boundary_lineage_digest",
            json!(""),
            "coverage_routine_boundary_lineage_missing",
        ),
        (
            "equivalence_status",
            json!("stale"),
            "coverage_routine_equivalence_unverified",
        ),
    ] {
        let failures = routine_failures(|_root, receipt| receipt[field] = value.clone());
        assert!(has(&failures, code), "{field}: {failures:?}");
    }

    let supported_complete = routine_failures(|_root, receipt| {
        receipt["supported_claim_classes"] = json!(["complete_coverage"]);
    });
    assert!(has(
        &supported_complete,
        "coverage_routine_supports_complete_coverage"
    ));

    let wrong_candidate = routine_failures(|_root, receipt| {
        receipt["target_revision"]["value"] = json!(crate::digest::ZERO);
    });
    assert!(has(
        &wrong_candidate,
        "coverage_routine_current_candidate_digest_mismatch"
    ));

    let wrong_lineage = routine_failures(|_root, receipt| {
        receipt["boundary_lineage_digest"] = json!("sha256:wrong");
    });
    assert!(has(
        &wrong_lineage,
        "coverage_routine_boundary_lineage_digest_mismatch"
    ));

    let false_strict_substitution = routine_failures(|_root, receipt| {
        receipt["equivalence_status"] = json!("verified_current_input_equivalent");
        receipt["strict_boundary_authority_status"] = json!("strict_boundary_not_run");
    });
    assert!(has(
        &false_strict_substitution,
        "coverage_routine_strict_boundary_authority_missing"
    ));

    let verified_strict_boundary = routine_failures(|_root, receipt| {
        receipt["equivalence_status"] = json!("verified_current_input_equivalent");
        receipt["strict_boundary_authority_status"] =
            json!("strict_boundary_current_input_equivalent");
        receipt["strict_boundary_receipt_path"] =
            json!("validation_artifacts/coverage/strict.json");
        receipt["strict_boundary_receipt_digest"] = json!(crate::digest::ZERO);
    });
    assert!(
        !has(
            &verified_strict_boundary,
            "coverage_routine_strict_boundary_authority_missing"
        ),
        "{verified_strict_boundary:?}"
    );

    let warm_cache = routine_failures(|_root, receipt| {
        receipt["coverage_cache_class"] = json!("warm_cache");
    });
    assert!(has(
        &warm_cache,
        "coverage_routine_cache_class_not_verified_local"
    ));
}

fn routine_failures(mut mutate: impl FnMut(&Path, &mut Value)) -> Vec<String> {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("coverage-routine-edge");
    super::write_coverage_root(
        &root,
        87.5,
        json!([{"path":"src/lib.rs","reason":"missed"}]),
    );
    let path = root.join(COVERAGE_RECEIPT_REL);
    let mut receipt = crate::json_boundary::read_json(&path).expect("receipt");
    make_routine(&mut receipt);
    mutate(&root, &mut receipt);
    crate::json_boundary::write_json(&path, &receipt).expect("write routine receipt");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let failures =
        super::super::super::routine::failures(&root, Path::new(COVERAGE_RECEIPT_REL), &candidate);
    fs::remove_dir_all(root).expect("cleanup routine edge");
    failures
}

fn make_routine(receipt: &mut Value) {
    receipt["claim_ceiling"] = json!("routine_repair_only");
    receipt["supported_claim_classes"] = json!(["routine_coverage_feedback"]);
    receipt["blocked_claim_classes"] = json!([
        "complete_coverage",
        "completion",
        "package_readiness",
        "review_readiness",
        "release_readiness",
        "final_packet_correctness",
        "update_goal_eligibility",
        "app_registry_or_reviewer_exposure"
    ]);
    receipt["coverage_cache_class"] = json!("retained_artifact_verified_local");
    receipt["cargo_version"] = json!("cargo test");
    receipt["rustc_version"] = json!("rustc test");
    let source_tree = receipt["source_tree_digest"]
        .as_str()
        .expect("source tree")
        .to_string();
    let manifest = receipt["coverage_manifest_digest"]
        .as_str()
        .expect("manifest")
        .to_string();
    let command = receipt["coverage_command_digest"]
        .as_str()
        .expect("command")
        .to_string();
    receipt["boundary_lineage_digest"] = json!(crate::digest::canonical_json(&json!({
        "mode": "routine_repair_only",
        "strict_boundary_mode": "full_clean_exact_100_uncovered_records_empty",
        "source_tree_digest": source_tree,
        "coverage_manifest_digest": manifest,
        "coverage_command_digest": command
    })));
    receipt["strict_boundary_authority_status"] = json!("strict_boundary_not_run");
    receipt["equivalence_status"] = json!("routine_feedback_only_no_strict_boundary_substitution");
}

fn has(failures: &[String], needle: &str) -> bool {
    failures.iter().any(|failure| failure.contains(needle))
}
