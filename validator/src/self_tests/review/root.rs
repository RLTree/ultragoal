use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent dir");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

#[test]
fn review_round_anchor_and_registry_checks_are_fail_closed() {
    let anchors = crate::review::round::anchor::values::AnchorValues {
        validator_path: "validation_artifacts/ultragoal-audit/validator-receipt.json".into(),
        review_target_path: "validation_artifacts/review-target.json".into(),
        archive_path: "validation_artifacts/archive.json".into(),
        validator_digest: crate::self_tests::boundaries::workspace_fixtures::sha('1'),
        review_target_digest: crate::self_tests::boundaries::workspace_fixtures::sha('2'),
        archive_digest: crate::self_tests::boundaries::workspace_fixtures::sha('3'),
        validator_run_id: "run-1".into(),
        package_digest: crate::self_tests::boundaries::workspace_fixtures::sha('4'),
        source_errors: vec!["archive source mismatch".into()],
        materiality_anchors: Vec::new(),
    };
    let mut failures = Vec::new();
    crate::review::round::anchor::values::anchor_errors(
        &json!({
            "status":"fail",
            "review_stage":"falsification",
            "anchor_policy":"validator_review_target_archive",
            "validator_receipt":{"path":"wrong","digest":crate::self_tests::boundaries::workspace_fixtures::sha('9'),"run_id":"run-2","package_digest":crate::self_tests::boundaries::workspace_fixtures::sha('8')}
        }),
        &anchors,
        &mut failures,
    );
    let errors = failures
        .iter()
        .map(|failure| failure.error.as_str())
        .collect::<Vec<_>>();
    assert!(errors.contains(&"review_round_anchor_source_mismatch"));
    assert!(errors.contains(&"review_round_evidence_incomplete"));
    assert!(errors.contains(&"review_round_missing_required_anchor"));
    assert!(errors.contains(&"review_round_stale_anchor_digest"));
    assert!(errors.contains(&"review_round_stale_anchor_path"));
    assert!(errors.contains(&"review_round_stale_validator_run"));
    assert!(errors.contains(&"review_round_stale_package_digest"));

    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("review-registry");
    std::fs::create_dir_all(root.join("validation_artifacts")).expect("registry dir");
    std::fs::create_dir_all(root.join("schemas")).expect("schema dir");
    std::fs::copy(
        crate::self_tests::boundaries::workspace_fixtures::repo_root()
            .join("schemas/codex-registry-exposure.schema.json"),
        root.join("schemas/codex-registry-exposure.schema.json"),
    )
    .expect("registry schema");
    failures.clear();
    crate::review::round::registry::exposure_errors(&root, &json!({}), &mut failures);
    assert_eq!(
        failures[0].error,
        "review_round_live_registry_exposure_missing"
    );
    failures.clear();
    crate::review::round::registry::exposure_errors(
        &root,
        &json!({"live_registry_exposure":{"path":"../escape.json","digest":crate::self_tests::boundaries::workspace_fixtures::sha('0')}}),
        &mut failures,
    );
    assert_eq!(
        failures[0].error,
        "review_round_live_registry_artifact_invalid"
    );

    let exposure_path = root.join("validation_artifacts/exposure.json");
    write_json(
        &exposure_path,
        &json!({
            "schema":"wrong",
            "source":"multi_agent_v1.tool_registry",
            "captured_at":"2026-06-25T00:00:00Z",
            "session_id":"s1",
            "round_id":"round-1",
            "agent_types":[{"role":"wrong","agent_type":"wrong","exposed":true}]
        }),
    );
    let digest = crate::digest::file(&exposure_path).expect("exposure digest");
    failures.clear();
    crate::review::round::registry::exposure_errors(
        &root,
        &json!({
            "generated_at":"2026-06-25T00:00:00Z",
            "round_id":"round-1",
            "live_registry_exposure":{"path":"validation_artifacts/exposure.json","digest":digest},
            "reviewers":[{"role":"claim-falsifier","live_spawn_receipt":{"source_thread_id":"other"}}]
        }),
        &mut failures,
    );
    let registry_errors = failures
        .iter()
        .map(|failure| failure.error.as_str())
        .collect::<Vec<_>>();
    assert!(registry_errors.contains(&"review_round_live_registry_artifact_malformed"));
    assert!(!registry_errors.contains(&"review_round_live_registry_stale"));
    assert!(!registry_errors.contains(&"review_round_live_registry_agent_missing"));

    failures.clear();
    crate::review::round::registry::row_agent_role_error(
        &json!({"role":"anything"}),
        "unknown-role",
        &mut failures,
    );
    assert!(failures.is_empty());
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn review_round_anchor_reader_binds_paths_digests_and_zero_fallback() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("review-anchor-reader");
    let anchor_dir = root.join("fixtures/review-round/anchors");
    std::fs::create_dir_all(&anchor_dir).expect("anchors");
    let validator = anchor_dir.join("validator-receipt.json");
    let target = anchor_dir.join("review-target-receipt.json");
    let archive = anchor_dir.join("archive-receipt.json");
    let package_digest = crate::self_tests::boundaries::workspace_fixtures::sha('a');
    write_json(
        &validator,
        &json!({
            "schema": "harness-ultragoal.validator-receipt.v1",
            "status": "pass",
            "run_id": "run-1",
            "target_revision": {
                "kind": "package_digest",
                "value": package_digest
            }
        }),
    );
    let validator_digest = crate::digest::file(&validator).expect("validator digest");
    write_json(
        &target,
        &json!({
            "schema":"harness-ultragoal.review-target-receipt.v1",
            "status":"pass",
            "package_digest":package_digest,
            "validator_receipt":{"path":"fixtures/review-round/anchors/validator-receipt.json","digest":validator_digest},
            "review_target_digest": crate::self_tests::boundaries::workspace_fixtures::sha('b')
        }),
    );
    write_json(
        &archive,
        &json!({
            "schema":"harness-ultragoal.distribution-archive-receipt.v1",
            "status":"pass",
            "archive": {"digest": crate::self_tests::boundaries::workspace_fixtures::sha('c')},
            "source": {"package_digest": package_digest}
        }),
    );
    let anchors = crate::review::round::anchor::values::fixture_anchor_values(&root);
    assert_eq!(
        anchors.validator_path,
        "fixtures/review-round/anchors/validator-receipt.json"
    );
    assert_eq!(
        anchors.review_target_path,
        "fixtures/review-round/anchors/review-target-receipt.json"
    );
    assert_eq!(
        anchors.archive_path,
        "fixtures/review-round/anchors/archive-receipt.json"
    );
    assert_eq!(
        anchors.validator_digest,
        crate::digest::file(&validator).unwrap()
    );
    assert_eq!(anchors.validator_run_id, "run-1");
    assert_eq!(anchors.package_digest, package_digest);
    assert_eq!(
        anchors.review_target_digest,
        crate::self_tests::boundaries::workspace_fixtures::sha('b')
    );
    assert_eq!(
        anchors.archive_digest,
        crate::self_tests::boundaries::workspace_fixtures::sha('c')
    );

    let zero = crate::review::round::anchor::values::fixture_anchor_values_for(
        &root,
        "fixtures/review-round/anchors/missing-validator.json",
        "fixtures/review-round/anchors/missing-review-target.json",
        "fixtures/review-round/anchors/missing-archive.json",
    );
    assert_eq!(zero.validator_digest, crate::digest::ZERO);
    assert_eq!(zero.package_digest, crate::digest::ZERO);
    std::fs::remove_dir_all(root).expect("cleanup anchors");
}
