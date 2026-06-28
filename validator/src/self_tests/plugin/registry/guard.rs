use serde_json::json;

#[test]
fn active_registry_claim_guard_accepts_only_cli_fail_closed_receipts() {
    let root = crate::self_tests::boundaries::support::temp_root("plugin-registry-guard-proof");
    super::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let current = crate::package::inventory::package_digest(&root).expect("digest");
    let raw_path =
        root.join("validation_artifacts/ultragoal-audit/active-registry-observation-current.json");
    super::write_json(&raw_path, &json!({"status":"fail","candidate":current}));
    let raw_digest = crate::digest::file(&raw_path).expect("raw digest");
    let receipt = super::fail_closed_registry_receipt(&current, &raw_digest);
    let store = crate::schema_catalog::load(&crate::self_tests::boundaries::support::repo_root());

    let failures =
        crate::audit::plugin::registry::value_claim_guard_failures(&root, &store, &receipt);
    assert!(failures.is_empty(), "{failures:?}");

    let live_raw_path = root.join("validation_artifacts/ultragoal-audit/live-registry-raw.json");
    super::write_json(
        &live_raw_path,
        &json!({"tool":"multi_agent_v1.tool_registry","candidate":current}),
    );
    let live_raw_digest = crate::digest::file(&live_raw_path).expect("live raw digest");
    let live_pass = super::live_registry_receipt(&current, &live_raw_digest);
    assert!(
        crate::audit::plugin::registry::value_claim_guard_failures(&root, &store, &live_pass)
            .is_empty()
    );

    let mut wrong_source = receipt.clone();
    wrong_source["source"] = json!("manual-pass-shaped-json");
    let failures =
        crate::audit::plugin::registry::value_claim_guard_failures(&root, &store, &wrong_source);
    assert!(
        failures
            .iter()
            .any(|failure| failure == "plugin_self_law_registry_guard_wrong_source"),
        "{failures:?}"
    );

    let mut missing_claim = receipt;
    missing_claim["failure"]["blocked_claim_classes"] = json!(["completion"]);
    let failures =
        crate::audit::plugin::registry::value_claim_guard_failures(&root, &store, &missing_claim);
    assert!(
        failures.iter().any(|failure| failure.contains(
            "plugin_self_law_registry_guard_missing_blocked_claim:update_goal_eligibility"
        )),
        "{failures:?}"
    );

    std::fs::remove_dir_all(root).expect("cleanup registry guard proof");
}
