#[test]
fn runtime_bound_custom_agent_red_packet_reaches_specific_plugin_failure() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let store = crate::schema_catalog::load(&root);
    let validator_artifacts =
        crate::audit::artifacts::validator_artifacts(&root).expect("validator artifacts");
    let validator_digests = crate::audit::artifacts::digest_map(&validator_artifacts);
    let packet = crate::json_boundary::read_json(
        &root.join("fixtures/red/custom-agent-app-visible-name-missing.json"),
    )
    .expect("custom agent red packet");
    let base_path = packet
        .get("base_fixture_path")
        .and_then(serde_json::Value::as_str)
        .expect("base path");
    let base = crate::json_boundary::read_json(&root.join(base_path)).expect("base fixture");
    let runtime_bound =
        crate::red::fixture::runtime::receipt::bind(&root, &base, &validator_digests);
    let bad = crate::claim_semantics::apply_patch(&runtime_bound, &packet["json_patch"])
        .expect("runtime-bound patch applies");
    let failures = crate::claim_semantics::semantic_failures(&bad, &root, &validator_digests);
    let first_failure = failures.first().expect("specific plugin failure");
    let first = format!("{}:{}", first_failure.check_id, first_failure.error);
    assert_eq!(
        first, "plugin-inventory-closure:custom_agent_app_visible_name_missing",
        "{failures:?}"
    );

    let observation = crate::red::fixture::observation::observe_materialized(
        &root,
        &store,
        &validator_digests,
        &packet,
        &packet["expected_failure"],
        &bad,
        base_path,
    );
    assert!(
        observation.ok,
        "check={} error={}",
        observation.check, observation.error
    );

    let report_rows = crate::red::fixtures::red_fixture_results(&root, &store, &validator_digests);
    let custom_agent_row = report_rows["custom-agent-app-visible-name-missing"].to_string();
    assert_eq!(
        report_rows["custom-agent-app-visible-name-missing"]["status"], "pass",
        "{custom_agent_row}"
    );
}
