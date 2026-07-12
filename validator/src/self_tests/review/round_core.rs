use serde_json::json;

#[test]
fn canonical_review_round_is_registry_blocked_while_materiality_fixtures_remain_valid() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let review_failures = crate::review::round::fixture_failures(&root);
    assert_eq!(
        review_failures,
        vec!["review-round fixture: review_round_live_registry_unavailable"]
    );
    let materiality_failures = crate::review::materiality::fixture_failures(&root);
    assert!(materiality_failures.is_empty(), "{materiality_failures:?}");
}

#[test]
fn review_round_core_reports_schema_and_unreadable_fixture_boundaries() {
    assert!(crate::review::round::is_review_round_fixture(
        "fixtures/review-round/valid/review-round-receipt.json",
    ));
    assert!(!crate::review::round::is_review_round_fixture(
        "fixtures/review-round/red/review-round-receipt.json",
    ));

    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("review-round-core");
    let fixture_errors = crate::review::round::fixture_failures(&root);
    assert!(
        fixture_errors
            .iter()
            .any(|error| error.contains("review-round fixture unreadable")),
        "{fixture_errors:?}"
    );

    let repo = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let store = crate::schema_catalog::load(&repo);
    let anchors = crate::review::round::anchor::values::fixture_anchor_values(&repo);
    let failures = crate::review::round::red_errors_with_anchors(
        &repo,
        &store,
        &json!({"schema":"wrong"}),
        &anchors,
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.error == "review_round_schema_invalid")
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn review_round_validate_files_reads_all_anchor_paths_fail_closed() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("review-round-validate-files");
    std::fs::create_dir_all(root.join("anchors")).expect("anchors");
    let receipt = root.join("receipt.json");
    std::fs::write(&receipt, "{}").expect("receipt");
    let validator = root.join("anchors/validator.json");
    let review_target = root.join("anchors/review-target.json");
    let archive = root.join("anchors/archive.json");
    std::fs::write(
        &validator,
        serde_json::to_vec(&json!({
            "run_id": "run",
            "target_revision": {"value": crate::self_tests::boundaries::workspace_fixtures::sha('a')}
        }))
        .expect("validator json"),
    )
    .expect("validator anchor");
    std::fs::write(
        &review_target,
        serde_json::to_vec(
            &json!({"review_target_digest": crate::self_tests::boundaries::workspace_fixtures::sha('b')}),
        )
        .expect("target json"),
    )
    .expect("review target anchor");
    std::fs::write(
        &archive,
        serde_json::to_vec(
            &json!({"archive": {"digest": crate::self_tests::boundaries::workspace_fixtures::sha('c')}}),
        )
        .expect("archive json"),
    )
    .expect("archive anchor");
    let anchors = crate::review::round::AnchorPaths {
        validator_receipt: validator,
        review_target_receipt: review_target,
        archive_receipt: archive,
    };
    let err = crate::review::round::validate_files(&root, &receipt, &anchors)
        .expect_err("invalid receipt rejected");
    assert!(
        err.contains("review_round_trusted_anchor_source_unavailable"),
        "{err}"
    );

    let missing_archive = crate::review::round::AnchorPaths {
        validator_receipt: anchors.validator_receipt.clone(),
        review_target_receipt: anchors.review_target_receipt.clone(),
        archive_receipt: root.join("anchors/missing-archive.json"),
    };
    let err = crate::review::round::validate_files(&root, &receipt, &missing_archive)
        .expect_err("missing archive anchor rejected");
    assert!(
        err.contains("review_round_trusted_anchor_source_unavailable"),
        "{err}"
    );
    std::fs::remove_dir_all(root).expect("cleanup review round validate files");
}

#[test]
fn review_round_anchor_sources_reject_invalid_sha_fields() {
    let mut failures = Vec::new();
    crate::review::round::anchor::sources::validate_anchor_sources(
        &crate::self_tests::boundaries::workspace_fixtures::sha('a'),
        &crate::self_tests::boundaries::workspace_fixtures::sha('b'),
        None,
        &json!({
            "status":"pass",
            "package_digest":crate::self_tests::boundaries::workspace_fixtures::sha('b'),
            "validator_receipt":{"digest":crate::self_tests::boundaries::workspace_fixtures::sha('a')},
            "review_target_digest":"not-sha"
        }),
        &json!({
            "status":"pass",
            "source":{"package_digest":crate::self_tests::boundaries::workspace_fixtures::sha('b')},
            "archive":{"digest":"sha256:ABC"}
        }),
        &mut failures,
    );
    assert!(failures.contains(&"review target digest is not a sha256 digest".to_string()));
    assert!(failures.contains(&"archive digest is not a sha256 digest".to_string()));
}
