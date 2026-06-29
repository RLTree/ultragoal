use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

#[test]
fn mandatory_law_surfaces_fail_missing_weak_and_digest_boundaries() {
    let root = crate::self_tests::boundaries::support::temp_root("mandatory-laws");
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    write_json(
        &root.join("templates/RED_FIXTURES.json"),
        &json!([{"id":"red-ok"}]),
    );
    std::fs::write(root.join("docs/evidence.md"), "old").expect("evidence");

    assert_eq!(
        crate::audit::mandatory::law::surfaces::value_failures(&root, &json!({})),
        vec!["mandatory_law_registry_missing_laws".to_string()]
    );
    let weak = json!({"laws":[{
        "law_id":"missing-law",
        "claim_ceiling_guard":"backlog",
        "valid_fixture_path":"fixtures/valid/missing.json",
        "red_fixture_ids":["red-missing"]
    }]});
    let failures = crate::audit::mandatory::law::surfaces::value_failures(&root, &weak);
    for expected in [
        "mandatory_law_weak_disposition:missing-law",
        "mandatory_law_missing_standards_row:missing-law",
        "mandatory_law_missing_source_obligation:missing-law",
        "mandatory_law_missing_foundational_trace:missing-law",
        "mandatory_law_missing_valid_fixture:missing-law",
        "mandatory_law_missing_red_fixture:missing-law:red-missing",
    ] {
        assert!(
            failures.contains(&expected.to_string()),
            "{expected}: {failures:?}"
        );
    }

    write_json(
        &root.join("templates/agent-standards/enforcement.json"),
        &json!({"rows":[{"id":"law-ok"}]}),
    );
    write_json(
        &root.join("docs/source-obligation-matrix.json"),
        &json!({"obligations":[{"id":"law-ok"}]}),
    );
    write_json(
        &root.join("docs/foundational-law-traceability.json"),
        &json!({"entries":[{"obligation_id":"law-ok"}]}),
    );
    write_json(
        &root.join("fixtures/valid/law-ok.json"),
        &json!({"ok":true}),
    );
    let valid_fixture = json!({"laws":[{
        "law_id":"law-ok",
        "valid_fixture_path":"fixtures/valid/law-ok.json",
        "red_fixture_ids":["red-ok"]
    }]});
    let failures = crate::audit::mandatory::law::surfaces::value_failures(&root, &valid_fixture);
    assert!(
        !failures
            .iter()
            .any(|failure| failure.contains("mandatory_law_missing_valid_fixture:law-ok")),
        "{failures:?}"
    );
    let no_valid_fixture = json!({"laws":[{
        "law_id":"law-ok",
        "red_fixture_ids":["red-ok"]
    }]});
    let failures = crate::audit::mandatory::law::surfaces::value_failures(&root, &no_valid_fixture);
    assert!(
        !failures
            .iter()
            .any(|failure| failure.contains("mandatory_law_missing_valid_fixture:law-ok")),
        "{failures:?}"
    );

    let receipt = json!({
        "law_id":"law-receipt",
        "schema":"wrong",
        "enforcement_status":"partial",
        "red_fixture_ids":[],
        "behavior_failure_modes":[],
        "law_specific": {"guard": false},
        "evidence_artifacts":[{}, {"path":"docs/evidence.md"}, {"path":"docs/evidence.md","digest":crate::self_tests::boundaries::support::sha('0')}]
    });
    let receipt_failures =
        crate::audit::mandatory::law::surfaces::receipt_value_failures(&root, &receipt);
    for expected in [
        "mandatory_law_wrong_schema:law-receipt",
        "mandatory_law_not_fail_closed:law-receipt",
        "mandatory_law_missing_red_fixtures:law-receipt",
        "mandatory_law_missing_failure_modes:law-receipt",
        "mandatory_law_specific_guard_not_enforced:law-receipt:guard",
        "mandatory_law_evidence_digest_mismatch:law-receipt",
        "mandatory_law_unknown_validator_check:law-receipt:",
    ] {
        assert!(
            receipt_failures.contains(&expected.to_string()),
            "{expected}: {receipt_failures:?}"
        );
    }
    let missing_specific = json!({
        "law_id":"no-specific",
        "schema":"harness-ultragoal.mandatory-law-surface-receipt.v1",
        "enforcement_status":"deterministic_fail_closed",
        "standards_row_id":"row",
        "source_obligation_id":"obligation",
        "foundational_trace_id":"trace",
        "validator_check_id":"check",
        "valid_fixture_path":"fixtures/valid/v.json",
        "claim_ceiling_guard":"blocks",
        "red_fixture_ids":["red-ok"],
        "behavior_failure_modes":["fails"]
    });
    assert_eq!(
        crate::audit::mandatory::law::surfaces::receipt_value_failures(&root, &missing_specific),
        vec!["mandatory_law_missing_specific_guards:no-specific".to_string()]
    );
    std::fs::remove_dir_all(root).expect("cleanup mandatory laws");
}

#[test]
fn mandatory_law_package_entrypoint_reads_registry_and_receipts() {
    let root = crate::self_tests::boundaries::support::temp_root("mandatory-law-package");
    let missing = crate::audit::mandatory::law::surfaces::package_failures(&root);
    assert!(
        missing
            .iter()
            .any(|failure| failure.contains("docs/mandatory-law-surfaces.json")),
        "{missing:?}"
    );

    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::create_dir_all(root.join("templates/agent-standards")).expect("standards");
    std::fs::create_dir_all(root.join("fixtures/valid")).expect("fixtures");
    write_json(
        &root.join("templates/RED_FIXTURES.json"),
        &json!([{"id":"red-ok"}]),
    );
    write_json(
        &root.join("templates/agent-standards/enforcement.json"),
        &json!({"rows":[{"id":"entry-law"}]}),
    );
    write_json(
        &root.join("docs/source-obligation-matrix.json"),
        &json!({"obligations":[{"id":"entry-law"}]}),
    );
    write_json(
        &root.join("docs/foundational-law-traceability.json"),
        &json!({"entries":[{"obligation_id":"entry-law"}]}),
    );
    write_json(&root.join("fixtures/valid/entry.json"), &json!({"ok":true}));
    write_json(
        &root.join("docs/mandatory-law-surfaces.json"),
        &json!({"laws":[{
            "law_id":"entry-law",
            "schema":"wrong",
            "enforcement_status":"partial",
            "valid_fixture_path":"fixtures/valid/entry.json",
            "red_fixture_ids":["red-ok"],
            "behavior_failure_modes":[],
            "law_specific":{"guard":false}
        }]}),
    );
    let failures = crate::audit::mandatory::law::surfaces::package_failures(&root);
    assert!(failures.contains(&"mandatory_law_wrong_schema:entry-law".to_string()));
    assert!(failures.contains(&"mandatory_law_not_fail_closed:entry-law".to_string()));
    assert!(
        !failures
            .iter()
            .any(|failure| failure == "mandatory_law_missing_valid_fixture:entry-law"),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup mandatory law package");
}
