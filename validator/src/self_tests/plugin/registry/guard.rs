use serde_json::json;

#[test]
fn active_registry_claim_guard_accepts_only_cli_fail_closed_receipts() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("plugin-registry-guard-proof");
    super::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let current = crate::package::inventory::package_digest(&root).expect("digest");
    let raw_path =
        root.join("validation_artifacts/ultragoal-audit/active-registry-observation-current.json");
    super::write_json(&raw_path, &super::fail_closed_raw_observation(&current));
    let raw_digest = crate::digest::file(&raw_path).expect("raw digest");
    let receipt = super::fail_closed_registry_receipt(&current, &raw_digest);
    let store = crate::schema_catalog::load(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root(),
    );

    let failures =
        crate::audit::plugin::registry::value_claim_guard_failures(&root, &store, &receipt);
    assert!(failures.is_empty(), "{failures:?}");

    let live_raw_path = root.join("validation_artifacts/ultragoal-audit/live-registry-raw.json");
    super::write_json(&live_raw_path, &super::raw_observation(&current));
    let live_raw_digest = crate::digest::file(&live_raw_path).expect("live raw digest");
    let live_pass = super::live_registry_receipt(&current, &live_raw_digest);
    let failures =
        crate::audit::plugin::registry::value_claim_guard_failures(&root, &store, &live_pass);
    assert!(
        failures
            .iter()
            .any(|failure| failure == "plugin_self_law_registry_positive_status_forbidden"),
        "{failures:?}"
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

#[test]
fn active_registry_claim_guard_requires_typed_capability_gap_record() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "plugin-registry-capability-gap",
    );
    super::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let current = crate::package::inventory::package_digest(&root).expect("digest");
    let raw_path =
        root.join("validation_artifacts/ultragoal-audit/active-registry-observation-current.json");
    super::write_json(&raw_path, &super::fail_closed_raw_observation(&current));
    let raw_digest = crate::digest::file(&raw_path).expect("raw digest");
    let receipt = super::fail_closed_registry_receipt(&current, &raw_digest);
    let store = crate::schema_catalog::load(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root(),
    );

    let mut missing_gap = receipt.clone();
    missing_gap
        .as_object_mut()
        .expect("object")
        .remove("capability_gap");
    let failures =
        crate::audit::plugin::registry::value_claim_guard_failures(&root, &store, &missing_gap);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("capability_gap")),
        "{failures:?}"
    );

    let mut weak_gap = receipt.clone();
    weak_gap["capability_gap"]["source_artifact"]["digest"] =
        json!(crate::self_tests::boundaries::workspace_fixtures::sha('9'));
    weak_gap["capability_gap"]["affected_law_ids"] =
        json!(["distribution-sharing-surface-claim-separation"]);
    weak_gap["capability_gap"]["affected_claim_ids"] = json!(["review_readiness"]);
    let failures =
        crate::audit::plugin::registry::value_claim_guard_failures(&root, &store, &weak_gap);
    assert!(
        failures.iter().any(|failure| failure
            .contains("plugin_self_law_registry_guard_capability_gap_source_artifact")),
        "{failures:?}"
    );
    assert!(
        failures.iter().any(|failure| {
            failure.contains(
        "plugin_self_law_registry_guard_capability_gap_missing_law:connector-capability-discovery"
    )
        }),
        "{failures:?}"
    );
    assert!(
        failures.iter().any(|failure| failure.contains(
            "plugin_self_law_registry_guard_capability_gap_missing_claim:update_goal_eligibility"
        )),
        "{failures:?}"
    );

    for (field, value, expected) in [
        ("schema", "wrong-schema", "capability_gap_schema"),
        (
            "source_session_id",
            "other-session",
            "capability_gap_session",
        ),
        (
            "observed_at",
            "2026-06-27T00:00:01Z",
            "capability_gap_observed_at",
        ),
        (
            "chosen_promotion_artifact",
            "validation_artifacts/review/final-packet-proof.json",
            "capability_gap_artifact",
        ),
        (
            "current_claim_ceiling",
            "live_registry_reviewer_exposure_proven",
            "capability_gap_ceiling",
        ),
        (
            "disposition",
            "closed_by_prose",
            "capability_gap_disposition",
        ),
    ] {
        let mut bad = receipt.clone();
        bad["capability_gap"][field] = json!(value);
        let failures =
            crate::audit::plugin::registry::value_claim_guard_failures(&root, &store, &bad);
        assert!(
            failures.iter().any(|failure| failure.contains(expected)),
            "{field}: {failures:?}"
        );
    }

    std::fs::remove_dir_all(root).expect("cleanup registry capability gap");
}
