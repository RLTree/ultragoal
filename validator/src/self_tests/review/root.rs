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
        validator_digest: crate::self_tests::boundaries::support::sha('1'),
        review_target_digest: crate::self_tests::boundaries::support::sha('2'),
        archive_digest: crate::self_tests::boundaries::support::sha('3'),
        validator_run_id: "run-1".into(),
        package_digest: crate::self_tests::boundaries::support::sha('4'),
        source_errors: vec!["archive source mismatch".into()],
    };
    let mut failures = Vec::new();
    crate::review::round::anchor::values::anchor_errors(
        &json!({
            "status":"fail",
            "round_phase":"sign_off",
            "anchor_policy":"validator_review_target_archive",
            "validator_receipt":{"path":"wrong","digest":crate::self_tests::boundaries::support::sha('9'),"run_id":"run-2","package_digest":crate::self_tests::boundaries::support::sha('8')}
        }),
        &anchors,
        &mut failures,
    );
    let errors = failures
        .iter()
        .map(|failure| failure.error.as_str())
        .collect::<Vec<_>>();
    assert!(errors.contains(&"review_round_anchor_source_mismatch"));
    assert!(errors.contains(&"review_round_status_not_pass"));
    assert!(errors.contains(&"review_round_missing_required_anchor"));
    assert!(errors.contains(&"review_round_stale_anchor_digest"));
    assert!(errors.contains(&"review_round_stale_anchor_path"));
    assert!(errors.contains(&"review_round_stale_validator_run"));
    assert!(errors.contains(&"review_round_stale_package_digest"));

    let root = crate::self_tests::boundaries::support::temp_root("review-registry");
    std::fs::create_dir_all(root.join("validation_artifacts")).expect("registry dir");
    failures.clear();
    crate::review::round::registry::exposure_errors(&root, &json!({}), &mut failures);
    assert_eq!(
        failures[0].error,
        "review_round_live_registry_exposure_missing"
    );
    failures.clear();
    crate::review::round::registry::exposure_errors(
        &root,
        &json!({"live_registry_exposure":{"path":"../escape.json","digest":crate::self_tests::boundaries::support::sha('0')}}),
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
            "agent_types":[{"agent_type":"wrong","persona":"wrong","exposed":true}]
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
            "reviewers":[{"persona":"p","live_spawn_receipt":{"source_thread_id":"other"}}]
        }),
        &mut failures,
    );
    let registry_errors = failures
        .iter()
        .map(|failure| failure.error.as_str())
        .collect::<Vec<_>>();
    assert!(registry_errors.contains(&"review_round_live_registry_artifact_malformed"));
    assert!(registry_errors.contains(&"review_round_live_registry_stale"));
    assert!(registry_errors.contains(&"review_round_live_registry_agent_missing"));

    failures.clear();
    crate::review::round::registry::row_agent_type_error(
        &json!({"agent_type":"anything"}),
        "unknown_persona",
        &mut failures,
    );
    assert!(failures.is_empty());
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn review_round_anchor_reader_binds_paths_digests_and_zero_fallback() {
    let root = crate::self_tests::boundaries::support::temp_root("review-anchor-reader");
    std::fs::create_dir_all(root.join("anchors")).expect("anchors");
    let validator = root.join("anchors/validator.json");
    let target = root.join("anchors/review-target.json");
    let archive = root.join("anchors/archive.json");
    write_json(
        &validator,
        &json!({
            "schema": "harness-ultragoal.validator-receipt.v1",
            "run_id": "run-1",
            "target_revision": {
                "kind": "package_digest",
                "value": crate::self_tests::boundaries::support::sha('a')
            }
        }),
    );
    write_json(
        &target,
        &json!({
            "review_target_digest": crate::self_tests::boundaries::support::sha('b')
        }),
    );
    write_json(
        &archive,
        &json!({
            "archive": {"digest": crate::self_tests::boundaries::support::sha('c')},
            "source": {"package_digest": crate::self_tests::boundaries::support::sha('a')}
        }),
    );
    let anchors = crate::review::round::anchor::values::AnchorValues::read(
        Some(&root),
        &validator,
        &target,
        &archive,
    )
    .expect("anchor values");
    assert_eq!(anchors.validator_path, "anchors/validator.json");
    assert_eq!(anchors.review_target_path, "anchors/review-target.json");
    assert_eq!(anchors.archive_path, "anchors/archive.json");
    assert_eq!(
        anchors.validator_digest,
        crate::digest::file(&validator).unwrap()
    );
    assert_eq!(anchors.validator_run_id, "run-1");
    assert_eq!(
        anchors.package_digest,
        crate::self_tests::boundaries::support::sha('a')
    );
    assert_eq!(
        anchors.review_target_digest,
        crate::self_tests::boundaries::support::sha('b')
    );
    assert_eq!(
        anchors.archive_digest,
        crate::self_tests::boundaries::support::sha('c')
    );

    let zero = crate::review::round::anchor::values::fixture_anchor_values_for(
        &root,
        "anchors/missing-validator.json",
        "anchors/missing-review-target.json",
        "anchors/missing-archive.json",
    );
    assert_eq!(zero.validator_digest, crate::digest::ZERO);
    assert_eq!(zero.package_digest, crate::digest::ZERO);
    std::fs::remove_dir_all(root).expect("cleanup anchors");
}

#[test]
fn product_fitness_and_target_fixture_boundaries_fail_closed() {
    let root = crate::self_tests::boundaries::support::temp_root("product-fitness");
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::create_dir_all(root.join("templates")).expect("templates");
    std::fs::create_dir_all(root.join("schemas")).expect("schemas");
    std::fs::create_dir_all(root.join("validation_artifacts/harness")).expect("harness");
    std::fs::write(
        root.join("docs/product-fitness-and-quality-in-use.md"),
        "This should be strict.",
    )
    .expect("law");
    std::fs::write(
        root.join("templates/PRODUCT_FITNESS.md"),
        "This may be strict.",
    )
    .expect("template");
    std::fs::write(
        root.join("schemas/product-fitness-receipt.schema.json"),
        "{}",
    )
    .expect("schema");
    std::fs::write(
        root.join("validation_artifacts/harness/product-fitness-receipt.json"),
        "{}",
    )
    .expect("receipt");
    let failures = crate::audit::product::fitness::package_failures(&root);
    assert!(
        failures
            .iter()
            .any(|item| item.starts_with("product_fitness_discretionary_language"))
    );
    assert!(
        failures
            .iter()
            .any(|item| item == "product_fitness_receipt_wrong_claim_id")
    );
    assert!(
        failures
            .iter()
            .any(|item| item == "product_fitness_receipt_malformed:schema")
    );
    assert!(
        failures
            .iter()
            .any(|item| item == "product_fitness_receipt_stale")
    );

    let missing = crate::target_fixtures::target_capability_failures(&root, &[]);
    assert!(
        missing
            .iter()
            .any(|item| item.starts_with("missing target-repo audit files"))
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}
