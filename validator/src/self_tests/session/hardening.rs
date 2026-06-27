use serde_json::json;

#[test]
fn session_log_hardening_reports_missing_and_unresolved_inventory() {
    let temp = crate::self_tests::boundaries::support::temp_root("session-hardening");
    std::fs::create_dir_all(temp.join("validation_artifacts/harness")).expect("harness dir");
    std::fs::create_dir_all(temp.join(".codex-plugin")).expect("plugin dir");
    std::fs::write(
        temp.join("plugin-manifest-draft.json"),
        "{\"version\":\"0.0.11\"}",
    )
    .expect("manifest");
    std::fs::write(
        temp.join(".codex-plugin/plugin.json"),
        "{\"version\":\"0.0.11\"}",
    )
    .expect("plugin");
    let store = crate::schema_catalog::load(&crate::self_tests::boundaries::support::repo_root());

    let missing = crate::audit::session_log_hardening::package_failures(&temp, &store);
    assert!(missing[0].starts_with("session_log_hardening_receipt_missing"));

    let receipt = json!({
        "schema": "harness-ultragoal.session-log-hardening-receipt.v1",
        "status": "pass",
        "generated_at": "2026-06-25T00:00:00Z",
        "candidate_version": "0.0.10",
        "source_root": "repo://test",
        "package_digest": crate::self_tests::boundaries::support::sha('c'),
        "audit_sources": [
            {"source_id":"s1","artifact":"a","source_kind":"session_log","timestamp_utc":"2026-06-25T00:00:00Z","summary":"x"},
            {"source_id":"s2","artifact":"a","source_kind":"session_log","timestamp_utc":"2026-06-25T00:00:00Z","summary":"x"},
            {"source_id":"s3","artifact":"a","source_kind":"packet_summary","timestamp_utc":"2026-06-25T00:00:00Z","summary":"x"},
            {"source_id":"s4","artifact":"a","source_kind":"validator_receipt","timestamp_utc":"2026-06-25T00:00:00Z","summary":"x"}
        ],
        "required_issue_classes": issue_classes(),
        "findings": [{
            "finding_id":"f1",
            "source_refs":["s1"],
            "timestamp_or_session_id":"s",
            "observed_gap":"gap",
            "affected_surface":"surface",
            "enforcement_status":"fixed",
            "required_repair":"repair",
            "claim_ceiling_impact":"impact",
            "artifact_types":["doc"]
        }],
        "packet_successor": {"path":"missing.json","purpose":"implemented_findings_and_remaining_blockers"},
        "active_registry_current_proof": "not_produced",
        "current_claim_ceiling": "does not support live app registry"
    });
    std::fs::write(
        temp.join("validation_artifacts/harness/session-log-hardening-receipt.json"),
        serde_json::to_vec(&receipt).expect("receipt json"),
    )
    .expect("receipt");
    let failures = crate::audit::session_log_hardening::package_failures(&temp, &store);
    assert!(
        failures
            .iter()
            .any(|item| item == "session_log_hardening_candidate_version_mismatch")
    );
    assert!(
        failures
            .iter()
            .any(|item| item.starts_with("session_log_hardening_source_kind_missing"))
    );
    assert!(
        failures
            .iter()
            .any(|item| item == "session_log_hardening_no_deterministic_fix")
    );
    assert!(
        failures
            .iter()
            .any(|item| item.starts_with("session_log_hardening_claim_ceiling_missing"))
    );
    std::fs::remove_dir_all(temp).expect("cleanup");
}

#[test]
fn session_log_hardening_covers_schema_version_and_fixed_validator_paths() {
    let temp = crate::self_tests::boundaries::support::temp_root("session-hardening-branches");
    std::fs::create_dir_all(temp.join("validation_artifacts/harness")).expect("harness dir");
    std::fs::create_dir_all(temp.join(".codex-plugin")).expect("plugin dir");
    let store = crate::schema_catalog::load(&crate::self_tests::boundaries::support::repo_root());
    std::fs::write(
        temp.join("validation_artifacts/harness/session-log-hardening-receipt.json"),
        "{}",
    )
    .expect("bad receipt");
    let schema_failures = crate::audit::session_log_hardening::package_failures(&temp, &store);
    assert!(
        schema_failures
            .iter()
            .any(|item| item.starts_with("session_log_hardening_schema"))
    );

    std::fs::write(temp.join("plugin-manifest-draft.json"), "{}").expect("manifest");
    std::fs::write(temp.join(".codex-plugin/plugin.json"), "{}").expect("plugin");
    let packet = temp.join("validation_artifacts/harness/session-packet.json");
    std::fs::write(&packet, "{}").expect("packet");
    let mut receipt = complete_receipt();
    receipt["required_issue_classes"][0]["class_id"] = json!("not_a_required_class");
    std::fs::write(
        temp.join("validation_artifacts/harness/session-log-hardening-receipt.json"),
        serde_json::to_vec(&receipt).expect("receipt"),
    )
    .expect("receipt");
    let failures = crate::audit::session_log_hardening::package_failures(&temp, &store);
    assert!(
        failures
            .iter()
            .any(|item| item.starts_with("session_log_hardening_version_unreadable")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|item| item.starts_with("session_log_hardening_issue_class_missing"))
    );
    assert!(
        !failures
            .iter()
            .any(|item| item == "session_log_hardening_no_deterministic_fix")
    );

    receipt["required_issue_classes"][0]["class_id"] = json!("stale_review_round_assumptions");
    receipt["required_issue_classes"]
        .as_array_mut()
        .expect("issue classes")
        .push(
            json!({"class_id":"duplicate_extra_class","disposition":"fixed","finding_ids":["f1"]}),
        );
    std::fs::write(
        temp.join("validation_artifacts/harness/session-log-hardening-receipt.json"),
        serde_json::to_vec(&receipt).expect("receipt"),
    )
    .expect("receipt");
    std::fs::write(
        temp.join("plugin-manifest-draft.json"),
        "{\"version\":\"0.0.12\"}",
    )
    .expect("manifest");
    std::fs::write(
        temp.join(".codex-plugin/plugin.json"),
        "{\"version\":\"0.0.11\"}",
    )
    .expect("plugin");
    let failures = crate::audit::session_log_hardening::package_failures(&temp, &store);
    assert!(
        failures
            .iter()
            .any(|item| item.contains("manifest version 0.0.12 != plugin version 0.0.11"))
    );
    std::fs::remove_dir_all(temp).expect("cleanup");
}

pub(crate) fn complete_receipt() -> serde_json::Value {
    json!({
        "schema": "harness-ultragoal.session-log-hardening-receipt.v1",
        "status": "pass",
        "generated_at": "2026-06-25T00:00:00Z",
        "candidate_version": "0.0.11",
        "source_root": "repo://test",
        "package_digest": crate::self_tests::boundaries::support::sha('c'),
        "audit_sources": [
            {"source_id":"s1","artifact":"a","source_kind":"session_log","timestamp_utc":"2026-06-25T00:00:00Z","summary":"x"},
            {"source_id":"s2","artifact":"a","source_kind":"chronicle_summary","timestamp_utc":"2026-06-25T00:00:00Z","summary":"x"},
            {"source_id":"s3","artifact":"a","source_kind":"packet_summary","timestamp_utc":"2026-06-25T00:00:00Z","summary":"x"},
            {"source_id":"s4","artifact":"a","source_kind":"validator_receipt","timestamp_utc":"2026-06-25T00:00:00Z","summary":"x"}
        ],
        "required_issue_classes": issue_classes(),
        "findings": [{
            "finding_id":"f1",
            "source_refs":["s1"],
            "timestamp_or_session_id":"s",
            "observed_gap":"gap",
            "affected_surface":"surface",
            "enforcement_status":"fixed",
            "required_repair":"repair",
            "claim_ceiling_impact":"impact",
            "artifact_types":["validator"]
        }],
        "packet_successor": {"path":"validation_artifacts/harness/session-packet.json","purpose":"implemented_findings_and_remaining_blockers"},
        "active_registry_current_proof": "not_produced",
        "current_claim_ceiling": "does not support live app registry; plugins ui unsupported; marketplace unsupported; current reviewer exposure unsupported; material review sign-off unsupported; real user product fitness unsupported; does not support 100% coverage"
    })
}

fn issue_classes() -> serde_json::Value {
    json!([
        {"class_id":"stale_review_round_assumptions","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"active_registry_proof_overclaim","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"disk_installed_cache_substituted_for_app_registry","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"reviewer_ready_without_current_exposure","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"product_fitness_docs_only","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"product::fitness::substitutions","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"source_install_cache_drift","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"stale_validator_receipts_or_red_fixtures","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"package_inventory_holes","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"packet_without_actionable_findings","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"standards_rows_prose_only","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"previous_followup_blocker_stale_missing_overclaim_terms","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"typed_boundary_self_audit","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"coverage_100_self_audit","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"line_cap_self_audit","disposition":"fixed","finding_ids":["f1"]}
    ])
}
