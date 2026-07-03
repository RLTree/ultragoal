use serde_json::{Value, json};

#[test]
fn session_log_hardening_reports_digest_and_cross_link_edges() {
    let store = crate::schema_catalog::load(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root(),
    );
    let digest_root = temp_root("session-hardening-digest-error");
    write_versions(&digest_root, json!(["missing.rs"]));
    write_receipt(
        &digest_root,
        crate::self_tests::session::hardening::complete_receipt(),
    );
    let digest_failures =
        crate::audit::session_log_hardening::package_failures(&digest_root, &store);
    assert!(
        digest_failures
            .iter()
            .any(|item| item.starts_with("session_log_hardening_package_digest_unreadable")),
        "{digest_failures:?}"
    );
    std::fs::remove_dir_all(digest_root).expect("cleanup digest root");

    let root = temp_root("session-hardening-cross-link");
    write_versions(&root, json!([]));
    std::fs::create_dir_all(root.join("validation_artifacts/harness")).expect("harness dir");
    std::fs::write(
        root.join("validation_artifacts/harness/session-packet.json"),
        "{}",
    )
    .expect("packet");
    let mut receipt = crate::self_tests::session::hardening::complete_receipt_for_root(&root);
    receipt["audit_sources"]
        .as_array_mut()
        .expect("sources")
        .retain(|row| row["source_id"] != "chronicle-registry-proof-1706");
    receipt["audit_sources"]
        .as_array_mut()
        .expect("sources")
        .push(json!({
            "source_id":"extra-session-source",
            "artifact":"a",
            "source_kind":"session_log",
            "timestamp_utc":"2026-06-25T00:00:00Z",
            "summary":"x"
        }));
    receipt["findings"][0]["implementation_status"] = json!("already_enforced");
    receipt["findings"][0]["source_refs"] = json!(["unknown-source"]);
    receipt["findings"][0]["signal"] = json!("");
    receipt["findings"][0]["claim_ids"] = json!([]);
    let duplicate = receipt["findings"][0].clone();
    receipt["findings"]
        .as_array_mut()
        .expect("findings")
        .push(duplicate);
    receipt["required_issue_classes"][0]["finding_ids"] = json!(["unknown-finding"]);
    write_receipt(&root, receipt);
    let failures = crate::audit::session_log_hardening::package_failures(&root, &store);
    for expected in [
        "session_log_hardening_source_id_missing:chronicle-registry-proof-1706",
        "session_log_hardening_duplicate_finding_id:f1",
        "session_log_hardening_implementation_status_mismatch:f1",
        "session_log_hardening_finding_field_missing:f1:signal",
        "session_log_hardening_finding_array_missing:f1:claim_ids",
        "session_log_hardening_source_ref_unknown:f1:unknown-source",
        "session_log_hardening_issue_finding_unknown:stale_review_round_assumptions:unknown-finding",
    ] {
        assert!(
            failures.iter().any(|item| item == expected),
            "{expected}: {failures:?}"
        );
    }
    std::fs::remove_dir_all(root).expect("cleanup root");
}

fn temp_root(name: &str) -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(name);
    std::fs::create_dir_all(root.join(".codex-plugin")).expect("plugin dir");
    std::fs::create_dir_all(root.join("validation_artifacts/harness")).expect("harness dir");
    root
}

fn write_versions(root: &std::path::Path, resources: Value) {
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({"version":"0.0.11","resources":resources}))
            .expect("manifest json"),
    )
    .expect("manifest");
    std::fs::write(
        root.join(".codex-plugin/plugin.json"),
        "{\"version\":\"0.0.11\"}",
    )
    .expect("plugin");
}

fn write_receipt(root: &std::path::Path, receipt: Value) {
    std::fs::write(
        root.join("validation_artifacts/harness/session-log-hardening-receipt.json"),
        serde_json::to_vec(&receipt).expect("receipt json"),
    )
    .expect("receipt");
}
