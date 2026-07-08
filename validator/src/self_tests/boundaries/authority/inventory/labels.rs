use serde_json::json;
use std::collections::BTreeSet;

#[test]
fn package_json_authority_rejects_goal_work_and_session_labels() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("authority-json-labels");
    let rel = "docs/improvement-loop-registry.json";
    std::fs::create_dir_all(root.join("docs")).expect("docs dir");
    std::fs::write(
        root.join(rel),
        serde_json::to_vec(&json!({
            "schema": "harness-ultragoal.improvement-loop-registry.v1",
            "trace_ids": ["gate-92-current-failure-observability"],
            "codex_handoff_ids": ["parent-session-source-local-repair-contract"],
            "implementation_change_ids": ["gate-93-research-law-registry"],
            "milestone_ids": ["research-law-registry-checkpoint"],
            "command_ids": ["agent-improvement-loop-slice"],
            "claim_impact": "Gate 92 remains blocked by stale observability proof",
            "generated_from": "observability gate prompt requirements"
        }))
        .expect("registry json"),
    )
    .expect("registry");

    let inventory = BTreeSet::from([rel.to_string()]);
    let failures = crate::audit::law::authority_surfaces::package_json_label_failures_for_test(
        &root, &inventory,
    );
    let text = failure_text(&failures);
    for expected in [
        "label=gate_number",
        "label=session_history_builder_contract",
        "label=checkpoint",
        "label=slice",
        "label=observability_product_closure",
    ] {
        assert!(
            text.contains(expected),
            "{expected} missing in {failures:?}"
        );
    }
    std::fs::remove_dir_all(root).expect("cleanup authority json labels");
}

#[test]
fn package_json_authority_allows_red_fixture_negative_examples() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("authority-json-red-fixture");
    let rel = "fixtures/red/namespace-goal-work-label-red.json";
    std::fs::create_dir_all(root.join("fixtures/red")).expect("red dir");
    std::fs::write(
        root.join(rel),
        serde_json::to_vec(&json!({
            "id": "namespace-goal-work-label-red",
            "bad_path": "validator/src/cli/observe/fitting/mod.rs",
            "bad_command": "parent-session-agent-improvement-loop-slice"
        }))
        .expect("red json"),
    )
    .expect("red fixture");

    let inventory = BTreeSet::from([rel.to_string()]);
    let failures = crate::audit::law::authority_surfaces::package_json_label_failures_for_test(
        &root, &inventory,
    );
    assert!(
        failures.is_empty(),
        "red fixture negative examples should not fail package authority labels: {failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup authority json red fixture");
}

#[test]
fn package_json_authority_ignores_root_scalars_without_authority_fields() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("authority-json-root-scalar");
    let rel = "docs/scalar-registry.json";
    std::fs::create_dir_all(root.join("docs")).expect("docs dir");
    std::fs::write(root.join(rel), "\"gate-92-current-failure-observability\"")
        .expect("scalar registry");

    let inventory = BTreeSet::from([rel.to_string()]);
    let failures = crate::audit::law::authority_surfaces::package_json_label_failures_for_test(
        &root, &inventory,
    );
    assert!(
        failures.is_empty(),
        "root scalars have no typed authority field and should not be classified as package authority labels: {failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup authority json root scalar");
}

#[test]
fn package_json_authority_scans_required_receipt_surfaces() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("authority-json-receipt");
    let rel = "validation_artifacts/coverage/coverage-receipt.json";
    std::fs::create_dir_all(root.join("validation_artifacts/coverage")).expect("receipt dir");
    std::fs::write(
        root.join(rel),
        serde_json::to_vec(&json!({
            "schema": "harness-ultragoal.coverage-receipt.v1",
            "claim_impact": "Gate 92 remains blocked by stale receipt evidence"
        }))
        .expect("receipt json"),
    )
    .expect("receipt");

    let inventory = BTreeSet::new();
    let failures = crate::audit::law::authority_surfaces::package_json_label_failures_for_test(
        &root, &inventory,
    );
    assert!(
        failure_text(&failures).contains("label=gate_number"),
        "required receipt surfaces must be scanned for product-opaque labels: {failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup authority json receipt");
}

#[test]
fn package_json_authority_does_not_promote_arbitrary_stale_receipts() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("authority-json-stale");
    let rel = "validation_artifacts/observability/stale-gate-receipt.json";
    std::fs::create_dir_all(root.join("validation_artifacts/observability"))
        .expect("observability receipt dir");
    std::fs::write(
        root.join(rel),
        serde_json::to_vec(&json!({
            "schema": "harness-ultragoal.observability-receipt.v1",
            "claim_impact": "Gate 92 remains blocked by stale downstream proof"
        }))
        .expect("receipt json"),
    )
    .expect("receipt");

    let inventory = BTreeSet::from([rel.to_string()]);
    let failures = crate::audit::law::authority_surfaces::package_json_label_failures_for_test(
        &root, &inventory,
    );
    assert!(
        failures.is_empty(),
        "arbitrary stale validation artifacts should not become active authority scans: {failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup authority json stale");
}

fn failure_text(failures: &[(String, String)]) -> String {
    failures
        .iter()
        .map(|(_, failure)| failure.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}
