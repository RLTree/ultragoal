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

    let mut receipt = complete_receipt();
    receipt["candidate_version"] = json!("0.0.10");
    for source in receipt["audit_sources"].as_array_mut().expect("sources") {
        source["source_kind"] = json!("session_log");
    }
    receipt["findings"][0]["artifact_types"] = json!(["doc"]);
    receipt["packet_successor"]["path"] = json!("missing.json");
    receipt["current_claim_ceiling"] = json!("does not support live app registry");
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
            .any(|item| item == "session_log_hardening_package_digest_mismatch")
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

pub(crate) fn complete_receipt_for_root(root: &std::path::Path) -> serde_json::Value {
    let digest = crate::package::inventory::package_digest(root).expect("package digest");
    complete_receipt_for_digest(&digest)
}

pub(crate) fn complete_receipt() -> serde_json::Value {
    complete_receipt_for_digest(&crate::self_tests::boundaries::support::sha('c'))
}

fn complete_receipt_for_digest(package_digest: &str) -> serde_json::Value {
    json!({
        "schema": "harness-ultragoal.session-log-hardening-receipt.v1",
        "status": "pass",
        "generated_at": "2026-06-25T00:00:00Z",
        "candidate_version": "0.0.11",
        "source_root": "repo://test",
        "package_digest": package_digest,
        "audit_sources": audit_sources(),
        "required_issue_classes": issue_classes(),
        "findings": [{
            "finding_id":"f1",
            "source_refs":["chronicle-packet-gap-1847"],
            "timestamp_or_session_id":"s",
            "signal":"session-log-regression-corpus",
            "affected_law_ids":["session-log-hardening"],
            "affected_package_surfaces":["review-packet"],
            "observed_bad_behavior":"bad path",
            "observed_gap":"gap",
            "affected_surface":"surface",
            "enforcement_status":"fixed",
            "required_repair":"repair",
            "fixture_ids":["session-log-hardening-valid-fixture"],
            "validator_ids":["session-log-hardening"],
            "receipt_ids":["validation_artifacts/harness/session-log-hardening-receipt.json"],
            "claim_ids":["review_readiness"],
            "claim_ceiling_impact":"impact",
            "artifact_types":["validator"],
            "implementation_status":"fixed",
            "evidence_digest": crate::self_tests::boundaries::support::sha('d')
        }],
        "packet_successor": {"path":"validation_artifacts/harness/session-packet.json","purpose":"implemented_findings_and_remaining_blockers"},
        "active_registry_current_proof": "not_produced",
        "current_claim_ceiling": "does not support live app registry; plugins ui unsupported; marketplace unsupported; current reviewer exposure unsupported; material review sign-off unsupported; real user product fitness unsupported; does not support 100% coverage"
    })
}

fn audit_sources() -> serde_json::Value {
    let ts = "2026-06-25T00:00:00Z";
    json!([
        {"source_id":"chronicle-product-fitness-0449","artifact":"a","source_kind":"chronicle_summary","timestamp_utc":ts,"summary":"x"},
        {"source_id":"chronicle-source-install-cache-0650","artifact":"a","source_kind":"chronicle_summary","timestamp_utc":ts,"summary":"x"},
        {"source_id":"chronicle-registry-proof-1706","artifact":"a","source_kind":"chronicle_summary","timestamp_utc":ts,"summary":"x"},
        {"source_id":"chronicle-packet-gap-1847","artifact":"a","source_kind":"chronicle_summary","timestamp_utc":ts,"summary":"x"},
        {"source_id":"chronicle-packet-completion-1906","artifact":"a","source_kind":"chronicle_summary","timestamp_utc":ts,"summary":"x"},
        {"source_id":"chronicle-product-fitness-ownership-2231","artifact":"a","source_kind":"chronicle_summary","timestamp_utc":ts,"summary":"x"},
        {"source_id":"chronicle-product-fitness-ownership-2232","artifact":"a","source_kind":"chronicle_summary","timestamp_utc":ts,"summary":"x"},
        {"source_id":"chronicle-namespace-progressive-disclosure-2236","artifact":"a","source_kind":"chronicle_summary","timestamp_utc":ts,"summary":"x"},
        {"source_id":"session-installed-drift-0907","artifact":"a","source_kind":"session_log","timestamp_utc":ts,"summary":"x"},
        {"source_id":"session-packet-run-1857","artifact":"a","source_kind":"session_log","timestamp_utc":ts,"summary":"x"},
        {"source_id":"session-self-audit-1935","artifact":"a","source_kind":"session_log","timestamp_utc":ts,"summary":"x"},
        {"source_id":"packet-summary-1900","artifact":"a","source_kind":"packet_summary","timestamp_utc":ts,"summary":"x"}
    ])
}

fn issue_classes() -> serde_json::Value {
    json!([
        {"class_id":"stale_review_round_assumptions","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"active_registry_proof_overclaim","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"disk_installed_cache_substituted_for_app_registry","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"reviewer_ready_without_current_exposure","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"product_fitness_docs_only","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"product_fitness_substitutions","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"product::fitness::substitutions","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"source_install_cache_drift","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"stale_validator_receipts_or_red_fixtures","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"package_inventory_holes","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"packet_without_actionable_findings","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"standards_rows_prose_only","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"previous_followup_blocker_stale_missing_overclaim_terms","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"typed_boundary_self_audit","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"coverage_100_self_audit","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"line_cap_self_audit","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"product_fitness_review_ownership","disposition":"fixed","finding_ids":["f1"]},
        {"class_id":"namespace_progressive_disclosure_under_enforced","disposition":"fixed","finding_ids":["f1"]}
    ])
}
