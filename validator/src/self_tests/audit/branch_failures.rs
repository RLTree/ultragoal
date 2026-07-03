use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent dir");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn write_text(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent dir");
    }
    std::fs::write(path, text).expect("write text");
}

fn contains(items: &[String], needle: &str) -> bool {
    items.iter().any(|item| item.contains(needle))
}

#[test]
fn mandatory_law_surfaces_and_tsv_evidence_fail_closed() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("mandatory-law-audit");
    write_json(&root.join("templates/RED_FIXTURES.json"), &json!([]));
    let registry = json!({"laws":[{
        "schema": "wrong",
        "law_id": "documentation-freshness",
        "enforcement_status": "partial",
        "standards_row_id": "",
        "source_obligation_id": "",
        "foundational_trace_id": "",
        "validator_check_id": "",
        "valid_fixture_path": "fixtures/missing.json",
        "claim_ceiling_guard": "claim-ceiling-only",
        "red_fixture_ids": ["missing-red"],
        "behavior_failure_modes": [],
        "law_specific": {"guard": false},
        "evidence_artifacts": [{}, {"path":"missing.txt","digest":crate::self_tests::boundaries::workspace_fixtures::sha('1')}]
    }]});
    let failures = crate::audit::mandatory::law::surfaces::value_failures(&root, &registry);
    assert!(contains(
        &failures,
        "mandatory_law_weak_disposition:documentation-freshness"
    ));
    assert!(contains(
        &failures,
        "mandatory_law_missing_standards_row:documentation-freshness"
    ));
    assert!(contains(
        &failures,
        "mandatory_law_missing_source_obligation:documentation-freshness"
    ));
    assert!(contains(
        &failures,
        "mandatory_law_missing_foundational_trace:documentation-freshness"
    ));
    assert!(contains(
        &failures,
        "mandatory_law_missing_valid_fixture:documentation-freshness"
    ));
    assert!(contains(
        &failures,
        "mandatory_law_missing_red_fixture:documentation-freshness:missing-red"
    ));

    let receipt_failures =
        crate::audit::mandatory::law::surfaces::receipt_value_failures(&root, &registry["laws"][0]);
    for expected in [
        "mandatory_law_wrong_schema:documentation-freshness",
        "mandatory_law_not_fail_closed:documentation-freshness",
        "mandatory_law_missing_field:documentation-freshness:standards_row_id",
        "mandatory_law_missing_failure_modes:documentation-freshness",
        "mandatory_law_specific_guard_not_enforced:documentation-freshness:guard",
        "mandatory_law_evidence_digest_mismatch:documentation-freshness",
    ] {
        assert!(
            contains(&receipt_failures, expected),
            "{expected}: {receipt_failures:?}"
        );
    }

    write_text(
        &root.join("standards.tsv"),
        "id\tsource_law\trequired_behavior\tenforcement_status\tgate_or_fixture_path\towner_lane\tclaim_ids_affected\tcurrent_status\tblocker_or_repair_action\trequired_execplan_refs\nrow1\tlaw\tbehavior\tmechanized\tgate\towner\tclaim\tcurrent\trepair\tE1\n",
    );
    write_text(
        &root.join("audit.tsv"),
        "standard_id\taudit_status\tevidence_path\tevidence_digest\taudited_at\tclaim_ceiling_impact\nrow2\tfail\tmissing.txt\tsha256:0000000000000000000000000000000000000000000000000000000000000000\t2026-06-25T00:00:00Z\tblocks\n",
    );
    let tsv_failures = crate::audit::agent::standards::tsv::checks::failures(
        &root,
        "standards.tsv",
        "audit.tsv",
        &json!({"rows":[
            {"id":"row1","enforcement_status":"deterministic_fail_closed","required_execplan_refs":["E2"]},
            {"id":"row3","enforcement_status":"deterministic_fail_closed","required_execplan_refs":[]}
        ]}),
    );
    for expected in [
        "agent_standards_tsv_json_mismatch:row1",
        "agent_standards_execplan_refs_drift:row1",
        "agent_standards_audit_missing:row1",
        "agent_standards_audit_missing:row3",
        "agent_standards_audit_not_pass:row2:fail",
    ] {
        assert!(
            contains(&tsv_failures, expected),
            "{expected}: {tsv_failures:?}"
        );
    }
    let audit_row = crate::audit::agent::standards::tsv::checks::audit_row_failures(
        &root,
        &json!({"standard_id":"row1","audit_status":"pass","evidence_path":"missing.txt","evidence_digest":crate::self_tests::boundaries::workspace_fixtures::sha('2')}),
    );
    assert!(contains(
        &audit_row,
        "agent_standards_audit_evidence_invalid:row1"
    ));
    std::fs::remove_dir_all(root).expect("cleanup mandatory law audit");
}

#[test]
fn plugin_self_laws_reject_stale_registry_coverage_and_line_cap() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("plugin-self-laws");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"version":"1.0.0"}),
    );
    write_json(
        &root.join(".codex-plugin/plugin.json"),
        &json!({"version":"2.0.0"}),
    );
    write_json(
        &root.join("validation_artifacts/coverage/coverage-receipt.json"),
        &json!({
            "coverage": {"policy":"partial","percent":99.0},
            "uncovered_records": [{"file":"validator/src/main.rs"}],
            "claim_ceiling": "withheld_or_blocked",
            "target_revision": {"value":"unavailable"}
        }),
    );
    write_json(
        &root.join("validation_artifacts/ultragoal-audit/active-registry-exposure-current.json"),
        &json!({
            "schema":"harness-ultragoal.multi-agent-registry-exposure.v1",
            "generated_at":"2026-06-24T00:00:00Z",
            "captured_at":"2026-06-24T00:00:00Z",
            "status":"fail",
            "source":"disk",
            "target_revision":{"kind":"package_digest","value":crate::self_tests::boundaries::workspace_fixtures::sha('b')},
            "claim_ceiling":"withheld_or_blocked",
            "session_id":"session",
            "round_id":"round",
            "agent_types":[]
        }),
    );
    write_text(&root.join("validator/src/too_long.rs"), &"x\n".repeat(251));
    let store = crate::schema_catalog::load(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root(),
    );
    let failures = crate::audit::plugin::laws::package_failures(&root, &store);
    for expected in [
        "plugin_self_law_version_mismatch",
        "plugin_self_law_coverage_policy_not_100_required",
        "plugin_self_law_coverage_not_100_percent",
        "plugin_self_law_coverage_has_uncovered_records",
        "plugin_self_law_coverage_claim_ceiling_not_complete",
        "plugin_self_law_coverage_target_revision_unavailable",
        "plugin_self_law_registry_target_digest_mismatch",
        "plugin_self_law_registry_guard_wrong_source",
        "plugin_self_law_registry_guard_capture_method_not_fail_closed",
        "plugin_self_law_registry_guard_failure_reason_missing",
        "plugin_self_law_registry_guard_missing_blocked_claim:app_registry_or_reviewer_exposure",
        "plugin_self_law_line_cap_exceeded:validator/src/too_long.rs:251",
    ] {
        assert!(contains(&failures, expected), "{expected}: {failures:?}");
    }
    std::fs::remove_dir_all(root).expect("cleanup plugin self laws");
}

#[test]
fn red_fixture_results_report_catalog_packet_and_base_failures() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("red-fixture-results");
    let store = crate::schema_catalog::load(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root(),
    );
    assert!(crate::red::fixtures::red_fixture_results(&root, &store, &BTreeMap::new()).is_empty());
    write_json(&root.join("templates/RED_FIXTURES.json"), &json!({}));
    assert!(crate::red::fixtures::red_fixture_results(&root, &store, &BTreeMap::new()).is_empty());

    write_text(&root.join("fixtures/red/malformed.json"), "{");
    write_json(
        &root.join("fixtures/valid/missing-red-base-fixture.json"),
        &json!({"ok":true}),
    );
    for (path, packet) in [
        (
            "fixtures/red/base-missing.json",
            json!({"base_fixture_path":"fixtures/valid/minimal-goal-run.json","expected_failure":{"check_id":"schema-valid","error":"base_fixture_missing"}}),
        ),
        (
            "fixtures/red/no-patch.json",
            json!({"base_fixture_path":"fixtures/valid/missing-red-base-fixture.json","expected_failure":{"check_id":"schema-valid","error":"red_fixture_json_patch_missing"}}),
        ),
        (
            "fixtures/red/bad-base.json",
            json!({"base_fixture_path":"fixtures/valid/not-allowed.json","expected_failure":{"check_id":"schema-valid","error":"invalid_base_fixture_path"}}),
        ),
    ] {
        write_json(&root.join(path), &packet);
    }
    write_json(
        &root.join("templates/RED_FIXTURES.json"),
        &json!([
            {"id":"escape","packet_path":"../escape.json","expected_failure":{"check_id":"schema-valid","error":"red_fixture_packet_path_invalid"}},
            {"id":"missing","packet_path":"fixtures/red/missing.json","expected_failure":{"check_id":"schema-valid","error":"red_fixture_packet_missing"}},
            {"id":"malformed","packet_path":"fixtures/red/malformed.json","expected_failure":{"check_id":"schema-valid","error":"red_fixture_packet_malformed_json"}},
            {"id":"base-missing","packet_path":"fixtures/red/base-missing.json","expected_failure":{"check_id":"schema-valid","error":"base_fixture_missing"}},
            {"id":"no-patch","packet_path":"fixtures/red/no-patch.json","expected_failure":{"check_id":"schema-valid","error":"red_fixture_json_patch_missing"}},
            {"id":"bad-base","packet_path":"fixtures/red/bad-base.json","expected_failure":{"check_id":"schema-valid","error":"invalid_base_fixture_path"}}
        ]),
    );
    let results = crate::red::fixtures::red_fixture_results(&root, &store, &BTreeMap::new());
    for (id, error) in [
        ("escape", "red_fixture_packet_path_invalid"),
        ("missing", "red_fixture_packet_missing"),
        ("malformed", "red_fixture_packet_malformed_json"),
        ("base-missing", "base_fixture_missing"),
        ("no-patch", "red_fixture_json_patch_missing"),
        ("bad-base", "invalid_base_fixture_path"),
    ] {
        assert_eq!(results[id]["observed_error"], error);
    }
    std::fs::remove_dir_all(root).expect("cleanup red fixtures");
}
