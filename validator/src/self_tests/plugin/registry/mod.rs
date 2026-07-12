use serde_json::{Value, json};
use std::path::Path;

mod fixture;
mod guard;
#[cfg(unix)]
mod raw_reader;

pub(super) use fixture::{
    fail_closed_raw_observation, fail_closed_registry_receipt, live_registry_receipt,
    raw_observation,
};

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

#[test]
fn active_registry_exposure_requires_live_same_surface_observation_provenance() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("plugin-registry-live-proof");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let current = crate::package::inventory::package_digest(&root).expect("digest");
    let raw_path = root.join("validation_artifacts/ultragoal-audit/live-registry-raw.json");
    write_json(&raw_path, &raw_observation(&current));
    let raw_digest = crate::digest::file(&raw_path).expect("raw digest");
    let receipt = live_registry_receipt(&current, &raw_digest);
    let store = crate::schema_catalog::load(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root(),
    );
    let failures = crate::audit::plugin::registry::value_failures(&root, &store, &receipt);
    assert!(
        failures
            .iter()
            .any(|failure| failure == "plugin_self_law_registry_positive_status_forbidden"),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("schema") && failure.contains("status")),
        "{failures:?}"
    );

    write_json(
        &raw_path,
        &json!({"tool":"multi_agent_v1.tool_registry","candidate":current}),
    );
    let minimal_digest = crate::digest::file(&raw_path).expect("minimal raw digest");
    let minimal = live_registry_receipt(&current, &minimal_digest);
    let failures = crate::audit::plugin::registry::value_failures(&root, &store, &minimal);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("raw_observation_wrong_schema")),
        "{failures:?}"
    );
    write_json(&raw_path, &raw_observation(&current));
    std::fs::write(&raw_path, "{").expect("malformed raw JSON");
    let malformed_digest = crate::digest::file(&raw_path).expect("malformed raw digest");
    let malformed = live_registry_receipt(&current, &malformed_digest);
    let failures = crate::audit::plugin::registry::value_failures(&root, &store, &malformed);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("raw_observation_malformed")),
        "{failures:?}"
    );
    write_json(&raw_path, &raw_observation(&current));

    let mut forged = receipt.clone();
    forged["raw_observation"]["digest"] =
        json!(crate::self_tests::boundaries::workspace_fixtures::sha('f'));
    let failures = crate::audit::plugin::registry::value_failures(&root, &store, &forged);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("raw_observation_digest_mismatch")),
        "{failures:?}"
    );

    let mut wrong_class = receipt.clone();
    wrong_class["raw_observation"]["path"] =
        json!("validation_artifacts/review/live-registry-raw.json");
    let failures = crate::audit::plugin::registry::value_failures(&root, &store, &wrong_class);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("raw_observation_path_invalid")),
        "{failures:?}"
    );

    let mut traversal = receipt;
    traversal["raw_observation"]["path"] =
        json!("validation_artifacts/ultragoal-audit/../review/final-packet.json");
    let failures = crate::audit::plugin::registry::value_failures(&root, &store, &traversal);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("raw_observation_path_invalid")),
        "{failures:?}"
    );

    let mut not_exposed_raw = raw_observation(&current);
    not_exposed_raw["registry_rows"][0]["exposed"] = json!(false);
    write_json(&raw_path, &not_exposed_raw);
    let not_exposed_digest = crate::digest::file(&raw_path).expect("not exposed raw digest");
    let not_exposed = live_registry_receipt(&current, &not_exposed_digest);
    let failures = crate::audit::plugin::registry::value_failures(&root, &store, &not_exposed);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("positive_proof_forbidden")),
        "{failures:?}"
    );
    write_json(&raw_path, &raw_observation(&current));

    let fail_raw_path =
        root.join("validation_artifacts/ultragoal-audit/active-registry-observation-current.json");
    write_json(&fail_raw_path, &fail_closed_raw_observation(&current));
    let fail_raw_digest = crate::digest::file(&fail_raw_path).expect("fail raw digest");
    let fail_closed = fail_closed_registry_receipt(&current, &fail_raw_digest);
    let schema_errors = crate::schema_catalog::schema_errors(
        &store,
        "codex-registry-exposure.schema.json",
        &fail_closed,
    );
    assert!(schema_errors.is_empty(), "{schema_errors:?}");
    let failures = crate::audit::plugin::registry::value_failures(&root, &store, &fail_closed);
    assert_eq!(
        failures,
        vec!["plugin_self_law_registry_live_exposure_unavailable"],
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup live registry proof");
}

#[test]
fn registry_rows_reject_legacy_mixed_duplicate_and_host_asserted_authority() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "plugin-registry-canonical-rows",
    );
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let current = crate::package::inventory::package_digest(&root).expect("digest");
    let raw_path = root.join("validation_artifacts/ultragoal-audit/live-registry-raw.json");
    write_json(&raw_path, &raw_observation(&current));
    let raw_digest = crate::digest::file(&raw_path).expect("raw digest");
    let receipt = live_registry_receipt(&current, &raw_digest);
    let store = crate::schema_catalog::load(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root(),
    );

    let mut legacy = receipt.clone();
    legacy["agent_types"][0]["agent_type"] = json!("claim-falsifier");
    legacy["agent_types"][0]["persona"] = json!("claim-falsifier");
    legacy["agent_types"][0]["custom_agent_path"] =
        json!("custom-agents/harness-contract-claim-falsifier.toml");
    let failures = crate::audit::plugin::registry::value_failures(&root, &store, &legacy);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("agent_legacy_keys")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("schema") && failure.contains("additional property")),
        "{failures:?}"
    );

    let mut duplicate = receipt.clone();
    duplicate["agent_types"][1] = duplicate["agent_types"][0].clone();
    let failures = crate::audit::plugin::registry::value_failures(&root, &store, &duplicate);
    assert!(
        failures.iter().any(|failure| {
            failure.contains("duplicate_path") || failure.contains("agent_missing")
        }),
        "{failures:?}"
    );

    let mut substituted = receipt.clone();
    substituted["agent_types"][0]["agent_manifest_path"] =
        json!(".codex/agents/security-reviewer.toml");
    let failures = crate::audit::plugin::registry::value_failures(&root, &store, &substituted);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("agent_mismatch")),
        "{failures:?}"
    );

    let mut host_asserted = receipt;
    host_asserted["agent_types"][0]["runtime_metadata_status"] = json!("host_exposed");
    host_asserted["agent_types"][0]["model"] = json!("gpt-inferred");
    let failures = crate::audit::plugin::registry::value_failures(&root, &store, &host_asserted);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("agent_runtime_unverified")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup canonical registry rows");
}
