use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn failures_for(root: &Path) -> Vec<String> {
    let store = crate::schema_catalog::load(&crate::self_tests::boundaries::support::repo_root());
    crate::audit::plugin::laws::package_failures(root, &store)
}

#[test]
fn plugin_self_laws_report_missing_versions_and_receipts() {
    let root = crate::self_tests::boundaries::support::temp_root("plugin-self-law-missing");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"version":""}),
    );
    write_json(
        &root.join(".codex-plugin/plugin.json"),
        &json!({"version":""}),
    );
    let failures = failures_for(&root);
    assert!(
        failures
            .iter()
            .any(|item| item.contains("plugin_self_law_version_missing"))
    );
    assert!(
        failures
            .iter()
            .any(|item| item.contains("plugin_self_law_json_missing_or_malformed"))
    );
    std::fs::remove_dir_all(root).expect("cleanup missing self laws");
}

#[test]
fn plugin_self_laws_report_registry_mismatch_and_not_current_fields() {
    let root = crate::self_tests::boundaries::support::temp_root("plugin-self-law-registry");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"version":"1.0.0"}),
    );
    write_json(
        &root.join(".codex-plugin/plugin.json"),
        &json!({"version":"1.0.0"}),
    );
    write_json(
        &root.join("validation_artifacts/coverage/coverage-receipt.json"),
        &json!({
            "coverage": {"policy":"100_percent_required","percent":100.0},
            "uncovered_records": [],
            "claim_ceiling": "supports_complete_claim",
            "target_revision": {"value": crate::self_tests::boundaries::support::sha('a')}
        }),
    );
    write_json(
        &root.join("validation_artifacts/ultragoal-audit/active-registry-exposure-current.json"),
        &json!({
            "schema": "harness-ultragoal.multi-agent-registry-exposure.v1",
            "generated_at": "2026-06-27T00:00:00Z",
            "captured_at": "2026-06-25T12:00:00Z",
            "status": "fail",
            "source": "multi_agent_v1.tool_registry",
            "target_revision": {
                "kind": "package_digest",
                "value": crate::self_tests::boundaries::support::sha('b')
            },
            "claim_ceiling": "withheld_or_blocked",
            "session_id": "session",
            "round_id": "round",
            "agent_types": [{
                "agent_type": "harness_contract_claim_falsifier",
                "persona": "wrong_persona",
                "custom_agent_path": "wrong.toml",
                "disk_cache_synced": false,
                "global_toml_present": false,
                "exposed": false
            }]
        }),
    );
    let failures = failures_for(&root);
    assert!(
        failures
            .iter()
            .any(|item| item.contains("plugin_self_law_registry_agent_mismatch"))
    );
    assert!(
        failures
            .iter()
            .any(|item| item.contains("plugin_self_law_registry_target_digest_mismatch"))
    );
    assert!(
        failures
            .iter()
            .any(|item| item.contains("plugin_self_law_registry_status_not_pass"))
    );
    assert!(
        failures
            .iter()
            .any(|item| item.contains("plugin_self_law_registry_claim_ceiling_not_live_surface"))
    );
    assert!(
        failures
            .iter()
            .any(|item| item.contains("plugin_self_law_registry_generated_capture_mismatch"))
    );
    assert!(
        failures
            .iter()
            .any(|item| item.contains("plugin_self_law_registry_missing_live_tool_issuer")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|item| { item.contains("plugin_self_law_registry_missing_tool_call_identity") }),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|item| item.contains("plugin_self_law_registry_capture_method_not_live")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|item| item.contains("plugin_self_law_registry_raw_observation_missing")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|item| item.contains("disk_cache_synced"))
    );
    assert!(
        failures
            .iter()
            .any(|item| item.contains("global_toml_present"))
    );
    assert!(failures.iter().any(|item| item.contains("exposed")));
    std::fs::remove_dir_all(root).expect("cleanup registry self laws");
}

#[test]
fn plugin_self_laws_report_unreadable_source_paths() {
    let root = crate::self_tests::boundaries::support::temp_root("plugin-self-law-unreadable");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"version":"1.0.0"}),
    );
    write_json(
        &root.join(".codex-plugin/plugin.json"),
        &json!({"version":"1.0.0"}),
    );
    std::fs::create_dir_all(root.join("validator/src")).expect("validator src");
    #[cfg(unix)]
    std::os::unix::fs::symlink("missing-target.rs", root.join("validator/src/broken.rs"))
        .expect("broken symlink");
    #[cfg(not(unix))]
    std::fs::write(root.join("validator/src/broken.rs"), "x").expect("fallback source");
    let failures = failures_for(&root);
    #[cfg(unix)]
    assert!(
        failures.iter().any(
            |item| item.contains("plugin_self_law_line_cap_unreadable:validator/src/broken.rs")
        ),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup unreadable self law");
}
