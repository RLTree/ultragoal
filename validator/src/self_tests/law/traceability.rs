use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn write_text(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, text).expect("write text");
}

fn contains(items: &[String], needle: &str) -> bool {
    items.iter().any(|item| item.contains(needle))
}

fn gardener_receipt(action: &str, severity: &str, artifact: Value) -> Value {
    json!({
        "schema": "harness-ultragoal.standards-gardening-receipt.v1",
        "status": "pass",
        "generated_at": "2026-06-25T00:00:00Z",
        "candidate_digest": crate::self_tests::boundaries::workspace_fixtures::sha('c'),
        "trigger_signal": {
            "signal_id": "sig",
            "severity": severity,
            "signal_kind": "standards_entropy",
            "summary": "summary",
            "source": "test"
        },
        "decision": {"accepted": true, "action": action, "rationale": "because"},
        "changed_artifacts": [artifact],
        "safeguards": {"deterministic_first": true, "no_hook_by_default": true},
        "claim_ceiling": "package_static_fixture_only"
    })
}

#[test]
fn foundational_law_trace_rejects_missing_unknown_stale_and_unbound_rows() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("law-trace");
    write_text(&root.join("source.md"), "source");
    write_json(
        &root.join("templates/agent-standards/enforcement.json"),
        &json!({"rows":[{"id":"law1"}]}),
    );
    write_json(
        &root.join("templates/RED_FIXTURES.json"),
        &json!([{"id":"red1"}]),
    );
    write_json(&root.join("fixtures/valid/v.json"), &json!({}));
    let matrix = json!({"obligations":[{"id":"law1"},{"id":"law2"}]});
    let trace = json!({"entries":[
        {},
        {
            "obligation_id":"law1",
            "source_artifact":{"path":"source.md","digest":crate::self_tests::boundaries::workspace_fixtures::sha('1')},
            "standards_row_id":"missing-standard",
            "validator_check_id":"missing-check",
            "red_fixture_id":"missing-red",
            "valid_fixture_id":"../escape.json"
        },
        {
            "obligation_id":"law1",
            "source_artifact":{"path":"source.md","digest":crate::digest::file(&root.join("source.md")).expect("digest")},
            "law_id":"law1",
            "standards_row_id":"law1",
            "validator_check_id":"schema-valid",
            "red_fixture_id":"red1",
            "valid_fixture_id":"fixtures/valid/v.json",
            "receipt_requirement":"receipt",
            "claim_ceiling_impact":"blocks"
        },
        {"obligation_id":"unknown"}
    ]});
    let failures = crate::audit::foundational_law_trace::value_failures(&root, &matrix, &trace);
    for expected in [
        "foundational_law_trace_missing_obligation:law2",
        "foundational_law_trace_missing_obligation_id",
        "foundational_law_trace_duplicate_obligation:law1",
        "foundational_law_trace_unknown_obligation:unknown",
        "foundational_law_trace_source_stale:law1",
        "foundational_law_trace_missing_law_id:law1",
        "foundational_law_trace_unknown_standard:law1:missing-standard",
        "foundational_law_trace_unknown_check:law1:missing-check",
        "foundational_law_trace_unknown_red_fixture:law1:missing-red",
        "foundational_law_trace_valid_fixture_missing:law1:../escape.json",
    ] {
        assert!(contains(&failures, expected), "{expected}: {failures:?}");
    }
    assert!(contains(
        &crate::audit::foundational_law_trace::failures(&root, &matrix),
        "foundational_law_trace_load_failed"
    ));
    std::fs::remove_dir_all(root).expect("cleanup law trace");
}

#[test]
fn foundational_law_trace_rejects_missing_entries_and_untyped_sources() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("law-trace-missing-entries");
    write_json(
        &root.join("templates/agent-standards/enforcement.json"),
        &json!({"rows":[]}),
    );
    write_json(&root.join("templates/RED_FIXTURES.json"), &json!([]));
    let matrix = json!({"obligations":[{"id":"law1"}]});
    assert_eq!(
        crate::audit::foundational_law_trace::value_failures(&root, &matrix, &json!({})),
        vec!["foundational_law_trace_entries_missing".to_string()]
    );

    let trace = json!({"entries":[{
        "obligation_id":"law1",
        "source_artifact":{"path":"","digest":crate::digest::ZERO},
        "law_id":"law1",
        "standards_row_id":"law1",
        "validator_check_id":"schema-valid",
        "red_fixture_id":"red1",
        "valid_fixture_id":"fixtures/valid/missing.json",
        "receipt_requirement":"receipt",
        "claim_ceiling_impact":"blocks"
    }]});
    let failures = crate::audit::foundational_law_trace::value_failures(&root, &matrix, &trace);
    assert!(contains(
        &failures,
        "foundational_law_trace_source_stale:law1"
    ));
    std::fs::remove_dir_all(root).expect("cleanup missing entries");
}

#[test]
fn standards_gardener_receipts_reject_missing_artifacts_semantics_and_stale_changes() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("standards-gardener");
    let store = crate::schema_catalog::load(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root(),
    );
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let current = crate::package::inventory::package_digest(&root).expect("digest");
    assert!(
        crate::audit::standards_gardening::value_failures(&json!({"obligations":[]})).is_empty()
    );
    assert_eq!(
        crate::audit::standards_gardening::value_failures(
            &json!({"obligations":[{"id":"standards-gardener-promotion"}]})
        ),
        vec!["standards_gardener_receipt_missing".to_string()]
    );
    write_json(
        &root.join("docs/source-obligation-matrix.json"),
        &json!({"obligations":[{"id":"standards-gardener-promotion","standards_gardening_receipt_path":"../escape.json"}]}),
    );
    assert!(contains(
        &crate::audit::standards_gardening::failures(&root, &store),
        "standards_gardener_receipt_artifact_invalid"
    ));

    let artifact_path = root.join("changed.json");
    write_json(
        &artifact_path,
        &json!({"generated_at":"2026-06-25T00:05:00Z"}),
    );
    let artifact = json!({
        "path":"changed.json",
        "digest":crate::self_tests::boundaries::workspace_fixtures::sha('2')
    });
    let low = gardener_receipt("validator_check", "low", artifact.clone());
    assert_eq!(
        crate::audit::standards_gardening::receipt_failures(&store, &low),
        vec!["standards_gardener_receipt_semantic_invalid".to_string()]
    );
    let hook = gardener_receipt("hook", "severe", artifact.clone());
    assert_eq!(
        crate::audit::standards_gardening::receipt_failures(&store, &hook),
        vec!["standards_gardener_receipt_semantic_invalid".to_string()]
    );
    let fresh = gardener_receipt("validator_check", "severe", artifact);
    let root_failures = crate::audit::standards_gardening::receipt_root_failures(&root, &fresh);
    assert!(contains(
        &root_failures,
        "standards_gardener_changed_artifact_digest_mismatch"
    ));
    assert!(contains(
        &root_failures,
        "standards_gardener_changed_artifact_after_receipt"
    ));
    let missing = gardener_receipt(
        "validator_check",
        "severe",
        json!({"path":"missing.json","digest":crate::self_tests::boundaries::workspace_fixtures::sha('3')}),
    );
    assert!(contains(
        &crate::audit::standards_gardening::receipt_root_failures(&root, &missing),
        "standards_gardener_changed_artifact_missing"
    ));
    let invalid_time = json!({
        "schema": "harness-ultragoal.standards-gardening-receipt.v1",
        "status": "pass",
        "generated_at": "not-a-timestamp",
        "candidate_digest": current,
        "trigger_signal": {"signal_id": "sig", "severity": "severe", "signal_kind": "standards_entropy", "summary": "summary", "source": "test"},
        "decision": {"accepted": true, "action": "validator_check", "rationale": "because"},
        "changed_artifacts": [],
        "safeguards": {"deterministic_first": true, "no_hook_by_default": true},
        "claim_ceiling": "package_static_fixture_only"
    });
    assert_eq!(
        crate::audit::standards_gardening::receipt_root_failures(&root, &invalid_time),
        vec!["standards_gardener_receipt_semantic_invalid".to_string()]
    );
    std::fs::remove_dir_all(root).expect("cleanup standards gardener");
}
