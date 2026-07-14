use super::write_json;
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn static_ready_examples_never_become_current_artifacts() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("package-run-ready-artifacts");
    std::fs::create_dir_all(root.join("examples/generated")).expect("generated");
    std::fs::create_dir_all(root.join("fixtures/valid")).expect("fixtures");
    for (name, run_id) in [("b", "future-run"), ("a", "run-current")] {
        write_json(
            &root.join(format!("examples/generated/READY_FOR_MERGE-{name}.json")),
            &json!({
                "lane_id":"duplicate",
                "ready":true,
                "commit":"abcdef0",
                "validator_run_id":run_id,
                "input_manifest_digest":crate::digest::ZERO,
                "claim_ceiling":[{"status":"proven_live"}]
            }),
        );
    }
    std::fs::write(
        root.join("examples/generated/READY_FOR_MERGE-malformed.json"),
        "{",
    )
    .expect("malformed static example");
    write_json(
        &root.join("fixtures/valid/embedded.json"),
        &json!({"ready_for_merge":{"lane_id":"different","ready":false}}),
    );

    assert!(crate::audit::package::run::ready_artifacts(&root, "run-current").is_empty());
    assert!(crate::audit::package::run::ready_artifacts(&root, "future-run").is_empty());
    std::fs::remove_dir_all(root).expect("cleanup ready artifacts");
}

#[test]
fn missing_ready_artifact_directory_is_empty() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "package-run-no-ready-artifacts",
    );
    std::fs::create_dir_all(&root).expect("root");
    assert!(crate::audit::package::run::ready_artifacts(&root, "run").is_empty());
    std::fs::remove_dir_all(root).expect("cleanup no ready artifacts");
}

#[test]
fn dependency_requires_real_ready_output() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "package-run-real-ready-required",
    );
    std::fs::create_dir_all(root.join("examples/generated")).expect("generated");
    std::fs::create_dir_all(root.join("fixtures/valid")).expect("fixtures");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let bundle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/valid/two-lane-ready-dependency.json"
    ))
    .expect("two-lane fixture");
    let fixture = root.join("fixtures/valid/two-lane-ready-dependency.json");
    write_json(&fixture, &bundle);
    write_json(
        &root.join("examples/generated/READY_FOR_MERGE-two-lane-ready-dependency.json"),
        &bundle["ready_for_merge_receipts"][0],
    );

    let mut failures = BTreeMap::new();
    crate::audit::package::run::semantic_valid_fixture_check(
        &root,
        &BTreeMap::new(),
        &[],
        &mut failures,
        &fixture,
    );
    assert!(
        failures
            .values()
            .flatten()
            .any(|failure| failure.contains("ready_receipt_not_validator_output")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup real ready requirement");
}
