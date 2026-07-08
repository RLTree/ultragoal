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
    receipt["boundary_lineage_digest"] = json!("sha256:boundary");
    receipt["equivalence_status"] = json!("verified_current_input_equivalent");
}

fn has(failures: &[String], needle: &str) -> bool {
    failures.iter().any(|failure| failure.contains(needle))
}
