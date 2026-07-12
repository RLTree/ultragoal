use serde_json::json;
use std::collections::BTreeMap;

fn write_json(path: &std::path::Path, value: &serde_json::Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

#[test]
fn package_run_valid_fixture_loader_reports_malformed_json_and_skips_non_json() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("package-run-fixtures");
    std::fs::create_dir_all(root.join("fixtures/valid")).expect("valid fixtures");
    std::fs::write(root.join("fixtures/valid/notes.txt"), "not json").expect("text fixture");
    std::fs::write(root.join("fixtures/valid/bad.json"), "{").expect("bad json");
    std::fs::write(
        root.join("fixtures/valid/claim.json"),
        serde_json::to_vec(&json!({
            "id":"CLAIM-PACKAGE-RUN",
            "title":"Completion claim",
            "description":"This claim has no required semantic receipts.",
            "status":"pass",
            "claim_ceiling_effect":"included",
            "product_applicability":{"user_facing":true}
        }))
        .expect("json"),
    )
    .expect("claim json");
    let mut failures = BTreeMap::new();
    crate::audit::package::run::semantic_valid_fixture_checks(
        &root,
        &BTreeMap::new(),
        &[],
        &mut failures,
    );
    let schema = failures.get("schema-valid").cloned().unwrap_or_default();
    assert!(
        schema.iter().any(|item| item.contains("bad.json")),
        "{schema:?}"
    );
    assert!(
        schema.iter().all(|item| !item.contains("notes.txt")),
        "{schema:?}"
    );
    assert!(
        failures
            .values()
            .flatten()
            .any(|item| item.contains("CLAIM-PACKAGE-RUN") || item.contains("claim.json"))
    );
    std::fs::remove_dir_all(root).expect("cleanup package run");
}

#[test]
fn package_run_entrypoint_writes_fail_closed_receipts_for_incomplete_package() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("package-run-entrypoint");
    for dir in [
        "examples/generated",
        "fixtures/valid",
        "schemas",
        "templates",
        "validation_artifacts/ultragoal-audit",
        "validator/src",
    ] {
        std::fs::create_dir_all(root.join(dir)).expect("dir");
    }
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({
            "name": "incomplete-harness-ultragoal",
            "version": "0.0.0",
            "resources": []
        }),
    );
    write_json(
        &root.join("schemas/schema-catalog.json"),
        &json!({"schemas":[]}),
    );
    write_json(
        &root.join("schemas/target-repo-receipt.schema.json"),
        &json!({"type":"object"}),
    );
    write_json(&root.join("templates/RED_FIXTURES.json"), &json!([]));
    write_json(
        &root.join("fixtures/valid/minimal-goal-run.json"),
        &json!({}),
    );
    let receipt = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
    let red_report = root.join("validation_artifacts/ultragoal-audit/red-fixture-report.json");

    let code = crate::audit::package::run::run(
        crate::audit::AuditOptions {
            root: root.clone(),
            receipt: receipt.clone(),
            red_report: Some(red_report.clone()),
            target_repo: None,
            mode: "source".to_string(),
            require_observability: false,
            require_product_cohesion: false,
            jobs: None,
            command_text: "ultragoal source audit --unit".to_string(),
        },
        red_report.clone(),
    )
    .expect("fail-closed package audit receipt");
    assert_eq!(code, 1);
    let receipt_json = crate::json_boundary::read_json(&receipt).expect("receipt");
    assert_eq!(receipt_json["status"], "fail");
    assert_eq!(
        receipt_json["generated_artifacts"][0]["artifact_type"],
        "red_fixture_report"
    );
    let red_json = crate::json_boundary::read_json(&red_report).expect("red report");
    assert_eq!(red_json["status"], "fail");
    std::fs::remove_dir_all(root).expect("cleanup package run entrypoint");
}

#[test]
fn red_report_status_is_independent_from_package_audit_status() {
    let mut red = BTreeMap::new();
    red.insert(
        "intended-failure".to_string(),
        json!({"status":"pass","packet_path":"fixtures/red/row.json","packet_digest":crate::digest::ZERO}),
    );
    assert_eq!(
        crate::audit::package::outputs::red_report_status(&red),
        "pass"
    );
    red.insert(
        "unexpected-pass".to_string(),
        json!({"status":"fail","packet_path":"fixtures/red/bad.json","packet_digest":crate::digest::ZERO}),
    );
    assert_eq!(
        crate::audit::package::outputs::red_report_status(&red),
        "fail"
    );
    assert_eq!(
        crate::audit::package::outputs::red_report_status(&BTreeMap::new()),
        "fail"
    );
}

#[test]
fn package_run_semantic_fixture_reports_absolute_outside_paths() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("package-run-outside");
    let outside = root.with_file_name("package-run-outside-valid.json");
    std::fs::write(&outside, "{").expect("outside json");
    let mut failures = BTreeMap::new();
    crate::audit::package::run::semantic_valid_fixture_check(
        &root,
        &BTreeMap::new(),
        &[],
        &mut failures,
        &outside,
    );
    let details = failures["schema-valid"].join("\n");
    assert!(
        details.contains(&outside.display().to_string()),
        "{details}"
    );
    let _ = std::fs::remove_file(outside);
}

#[test]
fn package_run_static_ready_examples_never_become_current_artifacts() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("package-run-ready-artifacts");
    std::fs::create_dir_all(root.join("examples/generated")).expect("generated");
    std::fs::create_dir_all(root.join("fixtures/valid")).expect("fixtures");
    write_json(
        &root.join("examples/generated/READY_FOR_MERGE-b.json"),
        &json!({
            "lane_id":"duplicate",
            "ready":true,
            "commit":"abcdef0",
            "validator_run_id":"future-run",
            "input_manifest_digest":crate::digest::ZERO,
            "claim_ceiling":[{"status":"proven_live"}]
        }),
    );
    write_json(
        &root.join("examples/generated/READY_FOR_MERGE-a.json"),
        &json!({
            "lane_id":"duplicate",
            "ready":true,
            "commit":"abcdef0",
            "validator_run_id":"run-current",
            "input_manifest_digest":crate::digest::ZERO,
            "claim_ceiling":[{"status":"proven_live"}]
        }),
    );
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
fn package_run_ready_artifacts_missing_directory_is_empty() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "package-run-no-ready-artifacts",
    );
    std::fs::create_dir_all(&root).expect("root");
    assert!(crate::audit::package::run::ready_artifacts(&root, "run").is_empty());
    std::fs::remove_dir_all(root).expect("cleanup no ready artifacts");
}

#[test]
fn package_run_requires_real_ready_output_when_dependency_needs_it() {
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
        "../../../../fixtures/valid/two-lane-ready-dependency.json"
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
