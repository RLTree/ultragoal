use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    std::fs::create_dir_all(path.parent().expect("json test path has parent")).expect("parent dir");
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

#[test]
fn semantic_receipt_loader_covers_inline_path_and_digest_boundaries() {
    let root = crate::self_tests::boundaries::support::temp_root("semantic-boundaries");
    std::fs::create_dir_all(root.join("receipts")).expect("receipts");
    let inline = json!({"claim_id":"C"});
    let loaded = crate::claim_semantics::semantic::receipt::loader::load(&root, &inline)
        .expect("inline receipt");
    assert!(loaded.inline);
    assert_eq!(loaded.value["claim_id"], "C");

    let receipt_path = root.join("receipts/ok.json");
    write_json(&receipt_path, &json!({"claim_id":"C2"}));
    let digest = crate::digest::file(&receipt_path).expect("digest");
    let loaded = crate::claim_semantics::semantic::receipt::loader::load(
        &root,
        &json!({"path":"receipts/ok.json","digest":digest}),
    )
    .expect("path receipt");
    assert!(!loaded.inline);
    assert_eq!(loaded.value["claim_id"], "C2");

    let err = crate::claim_semantics::semantic::receipt::loader::load(
        &root,
        &json!({"path":"receipts/ok.json","digest":crate::self_tests::boundaries::support::sha('0')}),
    )
    .err()
    .expect("digest mismatch rejected");
    assert!(err.contains("digest mismatch"));
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn red_catalog_and_row_boundaries_report_typed_failures() {
    let root = crate::self_tests::boundaries::support::temp_root("red-catalog");
    std::fs::create_dir_all(root.join("templates")).expect("templates");
    std::fs::create_dir_all(root.join("fixtures/red")).expect("red dir");
    std::fs::write(root.join("templates/RED_FIXTURES.json"), "{}").expect("catalog");
    let store = crate::schema_catalog::load(&crate::self_tests::boundaries::support::repo_root());
    let mut failures = BTreeMap::new();
    crate::audit::red::catalog::check(&root, &store, &mut failures);
    assert!(
        failures["red-fixture-coverage"]
            .iter()
            .any(|item| item.contains("red catalog must be an array"))
    );

    let packet = root.join("fixtures/red/p.json");
    write_json(
        &packet,
        &json!({"expected_failure":{"check_id":"schema-valid","error":"bad"}}),
    );
    write_json(
        &root.join("templates/RED_FIXTURES.json"),
        &json!([{
            "id":"p",
            "packet_path":"fixtures/red/p.json",
            "packet_digest":crate::self_tests::boundaries::support::sha('1'),
            "expected_failure":{"check_id":"schema-valid","error":"other"}
        }]),
    );
    failures.clear();
    crate::audit::red::catalog::check(&root, &store, &mut failures);
    let details = failures["red-fixture-coverage"].join("\n");
    assert!(details.contains("red catalog digest mismatch"));
    assert!(details.contains("red catalog expected_failure drift"));

    let expected = json!({"check_id":"claim-status-ceiling","error":"claim_overreach"});
    let row = crate::red::fixture::row::result_row(
        &root,
        "fixtures/red/p.json",
        &expected,
        "claim_overreach",
        crate::red::fixture::row::expected_status(&expected, "claim_overreach"),
        None,
        Some("claim-status-ceiling"),
    );
    assert_eq!(row["status"], "pass");
    assert_eq!(row["validator_exit"], 1);
    let invalid = crate::red::fixture::row::invalid_row(&json!({}), "schema_validation_failed");
    assert_eq!(invalid["packet_path"], "<invalid>");
    assert_eq!(
        crate::red::fixture::row::expected_check(&json!({})),
        "schema-valid"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn review_round_and_cli_parsers_reject_bad_boundaries() {
    assert!(crate::review::round::is_review_round_fixture(
        "fixtures/review-round/valid/review-round-receipt.json"
    ));
    assert!(!crate::review::round::is_review_round_fixture("other.json"));

    let temp = crate::self_tests::boundaries::support::temp_root("review-round-fixture-failure");
    write_json(
        &temp.join("fixtures/review-round/valid/review-round-receipt.json"),
        &json!({"schema":"bad-review-round","status":"fail"}),
    );
    let fixture_failures = crate::review::round::fixture_failures(&temp);
    assert!(!fixture_failures.is_empty());
    assert!(
        fixture_failures
            .iter()
            .all(|item| item.starts_with("review-round fixture:"))
    );
    std::fs::remove_dir_all(temp).expect("cleanup review fixture failure");

    assert!(crate::parse_command(&["unknown".to_string()]).is_err());
    let archive = crate::parse_command(&[
        "archive".to_string(),
        "--zip".to_string(),
        "a.zip".to_string(),
        "--receipt".to_string(),
        "r.json".to_string(),
        "--purpose".to_string(),
        "candidate_review_anchor".to_string(),
    ])
    .expect("archive parse");
    assert!(matches!(
        archive,
        crate::Command::Archive {
            archive_purpose,
            ..
        } if archive_purpose == "candidate_review_anchor"
    ));
    assert!(
        crate::parse_command(&["review-target".to_string()])
            .expect_err("missing receipt")
            .contains("missing required argument --receipt")
    );
}

#[test]
fn incomplete_package_audit_collects_fail_closed_package_branches() {
    let root = crate::self_tests::boundaries::support::temp_root("audit-missing-package");
    for dir in [
        "examples/generated",
        "fixtures",
        "schemas",
        "templates",
        "validation_artifacts/ultragoal-audit",
        "validator/src",
    ] {
        std::fs::create_dir_all(root.join(dir)).expect("create audit dir");
    }
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({
            "name": "incomplete-harness-ultragoal",
            "version": "0.0.0",
            "skills": [],
            "agents": [],
            "schemas": [],
            "fixtures": [],
            "authorable_templates": [],
            "generated_examples": [],
            "resources": []
        }),
    );
    std::fs::write(root.join("templates/RED_FIXTURES.json"), "{}").expect("red catalog");
    let receipt = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
    let red_report = root.join("validation_artifacts/ultragoal-audit/red-fixture-report.json");
    let code = crate::audit::run(crate::audit::AuditOptions {
        root: root.clone(),
        receipt: receipt.clone(),
        red_report: Some(red_report.clone()),
        target_repo: None,
        mode: "source".to_string(),
        require_observability: false,
        require_product_cohesion: false,
        command_text: "ultragoal source audit --test".to_string(),
    })
    .expect("incomplete source audit writes fail receipt");
    assert_eq!(code, 1);

    let value = crate::json_boundary::read_json(&receipt).expect("audit receipt");
    assert_eq!(value["status"], "fail");
    let checks = value["checks"].as_object().expect("checks object");
    for check in [
        "schema-valid",
        "red-fixture-coverage",
        "agent-standards-enforcement",
        "validator-execution-provenance",
        "target-repo-audit-capability",
        "plugin-inventory-closure",
    ] {
        assert_eq!(checks[check]["status"], "fail", "{check}");
    }
    let red = crate::json_boundary::read_json(&red_report).expect("red report");
    assert_eq!(red["status"], "fail");
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn review_round_validate_files_reports_anchor_failures() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let err = crate::review::round::validate_files(
        &root,
        &root.join("fixtures/review-round/valid/review-round-receipt.json"),
        &crate::review::round::AnchorPaths {
            validator_receipt: root.join("fixtures/review-round/anchors/validator-receipt.json"),
            review_target_receipt: root
                .join("fixtures/review-round/anchors/review-target-receipt.json"),
            archive_receipt: root.join("fixtures/review-round/anchors/archive-receipt.json"),
        },
    )
    .expect_err("stale review round rejected");
    assert!(err.contains("review_round_anchor_source_mismatch"));
}
