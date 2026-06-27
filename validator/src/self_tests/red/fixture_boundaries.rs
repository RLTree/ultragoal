use serde_json::{Value, json};
use std::collections::BTreeMap;

fn write_json(path: &std::path::Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("json parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json bytes")).expect("write json");
}

#[test]
fn red_fixture_observation_and_rows_fail_closed_on_no_failure_and_mismatch() {
    let root = crate::self_tests::boundaries::support::temp_root("red-fixture-boundaries");
    let observation = crate::red::fixture::package::observation(
        &root,
        &json!({"check_id":"source-card-freshness","error":"missing-error"}),
        &json!([]),
        "docs/source-cards.json",
    )
    .expect("source card observation");
    assert!(!observation.ok);
    assert_eq!(observation.error, "no_failure");

    assert_eq!(
        crate::red::fixture::row::expected_status(
            &json!({"error":"expected"}),
            "different-observed-error"
        ),
        "fail"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn red_fixture_no_failure_helpers_are_explicit() {
    assert_eq!(
        crate::red::fixture::observation::first_semantic_error(None),
        "no_failure"
    );
    assert_eq!(
        crate::red::fixture::review::round::first_review_error(None),
        "no_failure"
    );
    let row = crate::red::fixtures::base_fixture_json_result(
        Err("bad json".to_string()),
        std::path::Path::new("/repo"),
        "fixtures/red/bad.json",
        &json!({"check_id":"red-fixture-coverage","error":"base_fixture_malformed_json"}),
    )
    .expect_err("base fixture malformed row");
    assert_eq!(row["observed_error"], "base_fixture_malformed_json");
}

#[test]
fn red_fixture_materialization_and_mandatory_law_observations_fail_closed() {
    let root = crate::self_tests::boundaries::support::temp_root("red-fixture-materialization");
    std::fs::create_dir_all(root.join("templates")).expect("templates");
    std::fs::create_dir_all(root.join("fixtures/red")).expect("red fixtures");

    let mandatory_observation = crate::red::fixture::package::observation(
        &root,
        &json!({
            "check_id":"mandatory-law-surfaces",
            "error":"mandatory_law_registry_missing_laws"
        }),
        &json!({}),
        "docs/mandatory-law-surfaces.json",
    )
    .expect("mandatory-law observation");
    assert!(mandatory_observation.ok);
    assert_eq!(
        mandatory_observation.error,
        "mandatory_law_registry_missing_laws"
    );

    write_json(
        &root.join("fixtures/red/filesystem-kind.json"),
        &json!({
            "expected_failure": {
                "check_id": "red-fixture-coverage",
                "error": "red_filesystem_fixture_kind_unknown"
            },
            "base_fixture_path": "docs/source-cards.json",
            "filesystem_fixtures": [{"kind":"unknown"}],
            "json_patch": []
        }),
    );
    write_json(
        &root.join("templates/RED_FIXTURES.json"),
        &json!([{
            "id": "filesystem-kind",
            "packet_path": "fixtures/red/filesystem-kind.json",
            "expected_failure": {
                "check_id": "red-fixture-coverage",
                "error": "red_filesystem_fixture_kind_unknown"
            }
        }]),
    );
    let store = crate::schema_catalog::load(&crate::self_tests::boundaries::support::repo_root());
    let results = crate::red::fixtures::red_fixture_results(&root, &store, &BTreeMap::new());
    assert_eq!(
        results["filesystem-kind"]["observed_error"],
        "red_filesystem_fixture_kind_unknown"
    );

    #[cfg(unix)]
    {
        std::fs::create_dir_all(root.join("docs")).expect("docs");
        std::os::unix::fs::symlink(
            root.join("fixtures/red/filesystem-kind.json"),
            root.join("docs/source-cards.json"),
        )
        .expect("source cards symlink");
        write_json(
            &root.join("fixtures/red/base-symlink.json"),
            &json!({
                "expected_failure": {
                    "check_id": "red-fixture-coverage",
                    "error": "base_fixture_path_escapes_root"
                },
                "base_fixture_path": "docs/source-cards.json",
                "json_patch": []
            }),
        );
        write_json(
            &root.join("templates/RED_FIXTURES.json"),
            &json!([{
                "id": "base-symlink",
                "packet_path": "fixtures/red/base-symlink.json",
                "expected_failure": {
                    "check_id": "red-fixture-coverage",
                    "error": "base_fixture_path_escapes_root"
                }
            }]),
        );
        let results = crate::red::fixtures::red_fixture_results(&root, &store, &BTreeMap::new());
        assert_eq!(
            results["base-symlink"]["observed_error"],
            "base_fixture_path_escapes_root"
        );
    }

    std::fs::remove_dir_all(root).expect("cleanup red fixture materialization");
}

#[test]
fn red_fixture_package_observation_routes_all_package_surfaces() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let expected = json!({"check_id":"route-probe","error":"route-probe-error"});
    let bad = json!({});
    let package_paths = [
        "docs/source-cards.json",
        "docs/source-obligation-matrix.json",
        "docs/mandatory-law-surfaces.json",
        "docs/foundational-law-traceability.json",
        "templates/agent-standards/enforcement.json",
        "fixtures/agent-standards/valid/audit-pass-row.json",
        "fixtures/template-integrity/valid/template-integrity.json",
        "templates/.harness/coverage-manifest.json",
        "validation_artifacts/harness/fit-repo-receipt.json",
        "templates/validation_artifacts/harness/fit-repo-receipt.json",
        ".codex-plugin/plugin.json",
        "plugin-manifest-draft.json",
        "docs/namespace-class-registry.json",
        "fixtures/law-surfaces/valid/runtime-tool-identity-receipt.json",
        "fixtures/law-surfaces/valid/product-live-surface-receipt.json",
        "fixtures/law-surfaces/valid/transcript-quality-receipt.json",
        "fixtures/law-surfaces/valid/clean-checkout-command-discovery-receipt.json",
        "fixtures/law-surfaces/valid/restartable-execplan-receipt.json",
        "fixtures/law-surfaces/valid/memory-context-boundary-receipt.json",
        "fixtures/mandatory-law-surfaces/valid/cli-control-plane-authority.json",
        "docs/plugin-cohesion-manifest.json",
        "validation_artifacts/harness/plugin-product-journey-receipt.json",
        "validation_artifacts/harness/product-fitness-receipt.json",
        "fixtures/review-materiality/valid/review-materiality-receipt.json",
        "validation_artifacts/standards-gardener/current-standards-gardening-receipt.json",
    ];
    for path in package_paths {
        let observation = crate::red::fixture::package::observation(&root, &expected, &bad, path)
            .expect("package observation route");
        assert_eq!(observation.check, "route-probe");
    }
    assert!(
        crate::red::fixture::package::observation(&root, &expected, &bad, "unknown.json").is_none()
    );
}

#[test]
fn red_fixture_observation_routes_review_round_and_schema_layers() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let store = crate::schema_catalog::load(&root);

    let review_observation = crate::red::fixture::observation::observe_materialized(
        &root,
        &store,
        &BTreeMap::new(),
        &json!({"materialization":{"first_failure_must_match_expected":false}}),
        &json!({
            "check_id":"validator-execution-provenance",
            "error":"review_round_schema_invalid"
        }),
        &json!({"schema":"wrong"}),
        "fixtures/review-round/valid/review-round-receipt.json",
    );
    assert!(review_observation.ok);
    assert_eq!(review_observation.error, "review_round_schema_invalid");

    let schema_observation = crate::red::fixture::observation::observe_materialized(
        &root,
        &store,
        &BTreeMap::new(),
        &json!({"materialization":{"expected_validation_layer":"schema"}}),
        &json!({
            "check_id":"schema-valid",
            "error":"schema_validation_failed"
        }),
        &json!({"not_a_fixture_bundle":true}),
        "fixtures/valid/semantic-claim-boundary.json",
    );
    assert_eq!(schema_observation.check, "schema-valid");
    assert!(!schema_observation.error.is_empty());

    let semantic_observation = crate::red::fixture::observation::observe_materialized(
        &root,
        &store,
        &BTreeMap::new(),
        &json!({"materialization":{"first_failure_must_match_expected":false}}),
        &json!({
            "check_id":"claim-evidence-boundary",
            "error":"claim_evidence_missing_path"
        }),
        &json!({
            "claims":[{
                "id":"CLAIM",
                "claim_ceiling_effect":"included",
                "evidence":[{
                    "kind":"test_pass",
                    "surface":"ci",
                    "digest":crate::self_tests::boundaries::support::sha('a')
                }]
            }]
        }),
        "fixtures/valid/semantic-claim-boundary.json",
    );
    assert!(!semantic_observation.ok);
    assert!(!semantic_observation.check.is_empty());
}
