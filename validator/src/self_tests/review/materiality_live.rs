use serde_json::json;
use std::path::Path;

fn gates() -> Vec<&'static str> {
    vec![
        "anchor_existence_and_digest",
        "validator_receipt_status",
        "reviewer_registry_role_exposure",
        "stale_receipt_check",
        "package_private_artifact_hygiene",
        "readiness_validator",
        "claim_ceiling_check",
        "proof_surface_substitution_check",
    ]
}

#[test]
fn live_materiality_blocks_receipt_declared_delta_advisory_and_gate_runs() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let fixture = root.join("fixtures/review-round/valid/review-round-receipt.json");
    let receipt = crate::json_boundary::read_json(&fixture).expect("review fixture");
    let anchors = crate::review::round::anchor::values::fixture_anchor_values(&root);
    for (decision, trigger) in [
        ("DELTA_REVIEW_ALLOWED", "narrow_validator_only_change"),
        (
            "ADVISORY_REVIEW_ALLOWED",
            "advisory_only_no_candidate_change",
        ),
    ] {
        let mut forged = receipt.clone();
        forged["validator_receipt"]["path"] =
            json!("validation_artifacts/ultragoal-audit/validator-receipt.json");
        forged["review_target"]["path"] =
            json!("validation_artifacts/review/review-target-receipt.json");
        forged["archive"]["path"] =
            json!("validation_artifacts/review/candidate-archive-receipt.json");
        forged["materiality_gate"]["decision"] = json!(decision);
        forged["materiality_gate"]["material_triggers"] = json!([trigger]);
        forged["materiality_gate"]["deterministic_gates_run"] = json!(gates());
        let mut failures = Vec::new();
        crate::review::materiality::review_round_errors(&root, &forged, &anchors, &mut failures);
        let got = failures
            .iter()
            .map(|failure| failure.error.as_str())
            .collect::<Vec<_>>();
        assert!(
            got.contains(&"materiality_candidate_change_observation_unavailable"),
            "{decision}: {got:?}"
        );
        assert!(
            got.contains(&"materiality_executed_gate_observation_unavailable"),
            "{decision}: {got:?}"
        );
    }
}

#[test]
fn live_materiality_reuses_the_loaded_anchor_snapshot_after_path_mutation() {
    let repo = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("materiality-snapshot");
    let live = [
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
        "validation_artifacts/review/review-target-receipt.json",
        "validation_artifacts/review/candidate-archive-receipt.json",
    ];
    let fixtures = [
        "fixtures/review-round/anchors/validator-receipt.json",
        "fixtures/review-round/anchors/review-target-receipt.json",
        "fixtures/review-round/anchors/archive-receipt.json",
    ];
    for (source, target) in fixtures.iter().zip(live) {
        copy(&repo.join(source), &root.join(target));
    }
    let registry = "validation_artifacts/review/live-registry-exposure.json";
    copy(
        &repo.join("fixtures/review-round/anchors/live-registry-exposure.json"),
        &root.join(registry),
    );
    let paths = crate::review::round::AnchorPaths {
        validator_receipt: root.join(live[0]),
        review_target_receipt: root.join(live[1]),
        archive_receipt: root.join(live[2]),
    };
    let anchors = crate::review::round::anchor::values::AnchorValues::read(
        Some(&root),
        &paths.validator_receipt,
        &paths.review_target_receipt,
        &paths.archive_receipt,
    )
    .expect("initial live anchor snapshot");
    let mut receipt = crate::json_boundary::read_json(
        &repo.join("fixtures/review-round/valid/review-round-receipt.json"),
    )
    .expect("review fixture");
    for (key, path) in [
        ("validator_receipt", live[0]),
        ("review_target", live[1]),
        ("archive", live[2]),
    ] {
        receipt[key]["path"] = json!(path);
    }
    let bound = live
        .iter()
        .map(|path| json!({"path":path,"digest":crate::digest::file(&root.join(path)).expect("anchor digest")}))
        .collect::<Vec<_>>();
    receipt["materiality_gate"]["anchors_checked"] = json!(bound);
    let registry_digest = crate::digest::file(&root.join(registry)).expect("registry digest");
    receipt["live_registry_exposure"]["path"] = json!(registry);
    receipt["live_registry_exposure"]["digest"] = json!(registry_digest);
    receipt["materiality_gate"]["reviewer_registry_evidence"] =
        json!([{"path":registry,"digest":registry_digest}]);

    std::fs::write(
        &paths.validator_receipt,
        br#"{"schema":"harness-ultragoal.validator-receipt.v1","status":"pass"}"#,
    )
    .expect("mutate anchor after snapshot");
    let store = crate::schema_catalog::load(&repo);
    let failures = crate::review::round::red_errors_with_anchors(&root, &store, &receipt, &anchors);
    let errors = failures
        .iter()
        .map(|failure| failure.error.as_str())
        .collect::<Vec<_>>();
    assert!(
        !errors.contains(&"materiality_anchor_identity_invalid"),
        "{errors:?}"
    );
    assert!(
        !errors.contains(&"materiality_gate_evidence_unverified"),
        "{errors:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup snapshot test");
}

fn copy(source: &Path, target: &Path) {
    std::fs::create_dir_all(target.parent().expect("target parent")).expect("create parent");
    std::fs::copy(source, target).expect("copy fixture");
}
