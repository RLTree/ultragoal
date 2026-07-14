use super::{PREDECESSOR_ROOT, catalog, exact_repo, live_registry};
use crate::inventory::{ActiveStatus, AuthorityState};
use crate::repository_fixture::live_root;
use serde_json::{Value, json};
use std::fs;

const REPORT_PATH: &str = "REPORT.md";
const REPORT_DIGEST: &str = "e0a5534bf81ca2c8c0bc173081c82af1a75acc324ab291993a71c2dda35e58a5";

fn proposal_row() -> Value {
    json!({
        "context_id": "predecessor-plugin-proposal-report-2026-07-02",
        "path": REPORT_PATH,
        "sha256": REPORT_DIGEST,
        "authority": {"binding": false},
        "active_product_replaced": true,
        "replacement_contract_id": "harness-ultragoal-successor-contract-v2",
        "status": "replaced_historical_context",
        "preserve": true,
        "physical_deletion_authorized": false
    })
}

fn registry_with_proposal() -> Value {
    let mut registry = live_registry();
    registry["proposal_contexts"] = json!([proposal_row()]);
    registry
}

fn proposal_repo(label: &str) -> crate::repository_fixture::TestRepo {
    let repo = exact_repo(label);
    repo.write(
        REPORT_PATH,
        &fs::read(live_root().join(REPORT_PATH)).unwrap(),
    );
    repo.write(
        super::super::REGISTRY,
        &serde_json::to_vec(&registry_with_proposal()).unwrap(),
    );
    repo.commit();
    repo
}

fn has_rejection(repo: &crate::repository_fixture::TestRepo, code: &str) -> bool {
    catalog(repo)
        .unwrap()
        .findings()
        .iter()
        .any(|finding| finding.code == code)
}

#[test]
fn exact_predecessor_proposal_is_context_only() {
    let repo = proposal_repo("context-v2-proposal-exact");
    let catalog = catalog(&repo).unwrap();
    let report = catalog
        .entries()
        .iter()
        .find(|entry| entry.relative_path == REPORT_PATH)
        .unwrap();
    assert_eq!(report.authority_state, AuthorityState::Context);
    assert_eq!(report.active_status, ActiveStatus::ContextOnly);
    assert_eq!(report.kind, "non-authoritative-proposal-context");
    assert_eq!(report.digest_sha256, REPORT_DIGEST);
}

#[test]
fn proposal_tamper_fails_closed_without_partial_demotion() {
    let repo = proposal_repo("context-v2-proposal-tamper");
    repo.write(REPORT_PATH, b"tampered predecessor proposal");
    repo.commit();
    let catalog = catalog(&repo).unwrap();
    assert!(
        catalog
            .findings()
            .iter()
            .any(|finding| { finding.code == "non_authoritative_context_verification_failed" })
    );
    assert!(catalog.entries().iter().any(|entry| {
        entry.relative_path == REPORT_PATH
            && entry.authority_state == AuthorityState::Legacy
            && entry.active_status == ActiveStatus::Active
    }));
    assert!(catalog.entries().iter().any(|entry| {
        entry.relative_path == format!("{PREDECESSOR_ROOT}/README.md")
            && entry.authority_state == AuthorityState::Legacy
    }));
}

#[test]
fn registry_cannot_repoint_proposal_identity() {
    let repo = proposal_repo("context-v2-proposal-repoint");
    let mut registry = registry_with_proposal();
    registry["proposal_contexts"][0]["sha256"] = json!("0".repeat(64));
    repo.write(
        super::super::REGISTRY,
        &serde_json::to_vec(&registry).unwrap(),
    );
    repo.commit();
    assert!(has_rejection(
        &repo,
        "invalid_non_authoritative_context_registry"
    ));
}

#[test]
fn unknown_duplicate_and_extended_proposal_rows_are_rejected() {
    for (label, rows) in [
        (
            "unknown",
            json!([{"context_id":"unknown", "path":REPORT_PATH, "sha256":REPORT_DIGEST,
                "authority":{"binding":false}, "active_product_replaced":true,
                "replacement_contract_id":"harness-ultragoal-successor-contract-v2",
                "status":"replaced_historical_context", "preserve":true,
                "physical_deletion_authorized":false}]),
        ),
        ("duplicate", json!([proposal_row(), proposal_row()])),
    ] {
        let repo = proposal_repo(&format!("context-v2-proposal-{label}"));
        let mut registry = live_registry();
        registry["proposal_contexts"] = rows;
        repo.write(
            super::super::REGISTRY,
            &serde_json::to_vec(&registry).unwrap(),
        );
        repo.commit();
        assert!(has_rejection(
            &repo,
            "invalid_non_authoritative_context_registry"
        ));
    }

    let repo = proposal_repo("context-v2-proposal-extended");
    let mut registry = registry_with_proposal();
    registry["proposal_contexts"][0]["unexpected"] = json!(true);
    repo.write(
        super::super::REGISTRY,
        &serde_json::to_vec(&registry).unwrap(),
    );
    repo.commit();
    assert!(has_rejection(
        &repo,
        "invalid_non_authoritative_context_registry"
    ));
}

#[cfg(unix)]
#[test]
fn special_proposal_entry_is_rejected() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let repo = proposal_repo("context-v2-proposal-special");
    let report = repo.root.join(REPORT_PATH);
    fs::remove_file(&report).unwrap();
    let path = CString::new(report.as_os_str().as_bytes()).unwrap();
    // SAFETY: `path` is a live NUL-terminated CString and mode contains only permission bits.
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    assert!(has_rejection(
        &repo,
        "non_authoritative_context_verification_failed"
    ));
}
