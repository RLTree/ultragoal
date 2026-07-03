use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn red_fixture_materialization_and_mandatory_law_observations_fail_closed() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("red-fixture-materialization");
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

    super::write_json(
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
    super::write_json(
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
    let store = crate::schema_catalog::load(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root(),
    );
    let results = crate::red::fixtures::red_fixture_results(&root, &store, &BTreeMap::new());
    assert_eq!(
        results["filesystem-kind"]["observed_error"],
        "red_filesystem_fixture_kind_unknown"
    );

    #[cfg(unix)]
    base_symlink_observation_fails_closed(&root, &store);

    std::fs::remove_dir_all(root).expect("cleanup red fixture materialization");
}

#[cfg(unix)]
fn base_symlink_observation_fails_closed(
    root: &std::path::Path,
    store: &crate::schema_catalog::SchemaStore,
) {
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::os::unix::fs::symlink(
        root.join("fixtures/red/filesystem-kind.json"),
        root.join("docs/source-cards.json"),
    )
    .expect("source cards symlink");
    super::write_json(
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
    super::write_json(
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
    let results = crate::red::fixtures::red_fixture_results(root, store, &BTreeMap::new());
    assert_eq!(
        results["base-symlink"]["observed_error"],
        "base_fixture_path_escapes_root"
    );
}
