use serde_json::Value;
use std::collections::BTreeMap;

#[test]
fn red_fixture_runtime_binding_exposes_intended_semantic_failure() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let store = crate::schema_catalog::load(&root);
    let validator_digests = validator_artifact_digest(&root);
    let provenance_packet = crate::json_boundary::read_json(
        &root.join("fixtures/red/fabricated-validator-receipt.json"),
    )
    .expect("provenance packet");
    let provenance_base_path = provenance_packet
        .get("base_fixture_path")
        .and_then(Value::as_str)
        .expect("provenance base path");
    let provenance_base =
        crate::json_boundary::read_json(&root.join(provenance_base_path)).expect("base fixture");
    let provenance_bound =
        crate::red::fixtures::runtime_bound_bundle(&root, &provenance_base, &validator_digests);
    let provenance_bad =
        crate::claim_semantics::apply_patch(&provenance_bound, &provenance_packet["json_patch"])
            .expect("provenance patch applies");
    let schema_errors =
        crate::red::fixture::schema::errors(&store, &provenance_packet, &provenance_bad);
    assert!(schema_errors.is_empty(), "schema={schema_errors:?}");
    let provenance_observation = crate::red::fixture::observation::observe_materialized(
        &root,
        &store,
        &validator_digests,
        &provenance_packet,
        &provenance_packet["expected_failure"],
        &provenance_bad,
        provenance_base_path,
    );
    assert!(
        provenance_observation.ok,
        "check={} error={}",
        provenance_observation.check, provenance_observation.error
    );
    assert_eq!(
        provenance_observation.error,
        "validator_receipt_not_runtime_provenance"
    );

    let packet = crate::json_boundary::read_json(
        &root.join("fixtures/red/actor-validation-older-than-heartbeat.json"),
    )
    .expect("red packet");
    let base_path = packet
        .get("base_fixture_path")
        .and_then(Value::as_str)
        .expect("base path");
    let base = crate::json_boundary::read_json(&root.join(base_path)).expect("base fixture");
    let expected = &packet["expected_failure"];

    let runtime_bound =
        crate::red::fixtures::runtime_bound_bundle(&root, &base, &validator_digests);
    let bad = crate::claim_semantics::apply_patch(&runtime_bound, &packet["json_patch"])
        .expect("runtime-bound patch applies");
    let observation = crate::red::fixture::observation::observe_materialized(
        &root,
        &store,
        &validator_digests,
        &packet,
        expected,
        &bad,
        base_path,
    );
    assert!(observation.ok);
    assert_eq!(observation.check, "lane-actor-binding");
    assert_eq!(observation.error, "actor_identity_older_than_heartbeat");

    let mut fallback = base.clone();
    fallback["ready_for_merge"]
        .as_object_mut()
        .expect("ready object")
        .remove("validator_run_id");
    fallback["validator_receipt"]["run_id"] = serde_json::json!("receipt-run-id");
    let fallback_bound =
        crate::red::fixtures::runtime_bound_bundle(&root, &fallback, &validator_digests);
    assert_eq!(
        fallback_bound["validator_receipt"]["run_id"],
        "receipt-run-id"
    );
}

#[test]
fn generated_artifacts_fail_closed_when_runtime_inputs_are_missing() {
    let root =
        crate::self_tests::boundaries::support::temp_root("runtime-generated-artifacts-missing");
    std::fs::create_dir_all(&root).expect("temp root");

    assert_eq!(
        crate::red::fixture::runtime::artifact::digest_or_zero(&root, "../escape.json"),
        crate::digest::ZERO
    );
    let artifacts =
        crate::red::fixture::runtime::artifact::generated_artifacts(&root, "runtime-test");
    assert!(artifacts.is_empty());

    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn generated_artifacts_classify_schema_required_leaf_paths() {
    let root =
        crate::self_tests::boundaries::support::temp_root("runtime-generated-artifacts-schema");
    std::fs::create_dir_all(root.join("schemas")).expect("schema root");
    std::fs::write(
        root.join("schemas/validator-receipt.schema.json"),
        serde_json::json!({
            "anyOf": [
                {"properties": {"path": {"pattern": "(^|/)custom-artifact.json$"}}},
                {"properties": {"path": {"pattern": "(^|/)READY_FOR_MERGE-custom.json$"}}}
            ]
        })
        .to_string(),
    )
    .expect("schema");

    let artifacts =
        crate::red::fixture::runtime::artifact::generated_artifacts(&root, "runtime-test");
    assert!(
        artifacts
            .iter()
            .any(|row| row.get("path").and_then(Value::as_str) == Some("custom-artifact.json"))
    );
    assert!(artifacts.iter().any(|row| {
        row.get("path").and_then(Value::as_str) == Some("READY_FOR_MERGE-custom.json")
            && row.get("artifact_type").and_then(Value::as_str) == Some("ready_for_merge")
    }));

    std::fs::remove_dir_all(root).expect("cleanup");
}

#[cfg(unix)]
#[test]
fn runtime_bound_receipt_falls_back_when_input_refs_are_unreadable() {
    let root = crate::self_tests::boundaries::support::temp_root("runtime-input-ref-fallback");
    std::fs::create_dir_all(&root).expect("temp root");
    std::os::unix::fs::symlink("missing-target", root.join("plugin-manifest-draft.json"))
        .expect("manifest symlink");
    let validator_digests =
        validator_artifact_digest(&crate::self_tests::boundaries::support::repo_root());
    let bundle = serde_json::json!({"schema": "harness-ultragoal.fixture-bundle.v1"});

    let bound = crate::red::fixtures::runtime_bound_bundle(&root, &bundle, &validator_digests);
    assert_eq!(
        bound["validator_receipt"]["input_digests"][0]["path"],
        "fixtures/valid/minimal-goal-run.json"
    );

    std::fs::remove_dir_all(root).expect("cleanup");
}

#[cfg(unix)]
#[test]
fn runtime_input_digest_falls_back_for_unsafe_file_refs() {
    let root = crate::self_tests::boundaries::support::temp_root("runtime-input-digest-zero");
    std::fs::create_dir_all(root.join("schemas")).expect("schemas");
    std::fs::create_dir_all(root.join("templates")).expect("templates");
    std::fs::create_dir_all(root.join("fixtures/valid")).expect("fixtures");
    std::fs::write(root.join("plugin-manifest-draft.json"), "{}").expect("manifest");
    std::fs::hard_link(
        root.join("plugin-manifest-draft.json"),
        root.join("schemas/schema-catalog.json"),
    )
    .expect("schema hardlink");
    for rel in [
        "templates/RED_FIXTURES.json",
        "fixtures/valid/minimal-goal-run.json",
    ] {
        std::fs::write(root.join(rel), "[]").expect("input file");
    }
    let mut cache = BTreeMap::new();
    let rows = crate::red::fixture::runtime::receipt::input_digests_with_cache(&root, &mut cache);
    assert!(rows.iter().any(|row| {
        row.get("path").and_then(Value::as_str) == Some("schemas/schema-catalog.json")
            && row.get("digest").and_then(Value::as_str) == Some(crate::digest::ZERO)
    }));
    std::fs::remove_dir_all(root).expect("cleanup");
}

fn validator_artifact_digest(root: &std::path::Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    out.insert(
        "validator/src/red/fixtures.rs".to_string(),
        crate::digest::file(&root.join("validator/src/red/fixtures.rs")).expect("fixture digest"),
    );
    out
}
