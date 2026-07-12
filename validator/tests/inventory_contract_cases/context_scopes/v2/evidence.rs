use super::*;

fn evidence_failed(catalog: &crate::inventory::AuthorityCatalog) -> bool {
    catalog
        .findings()
        .iter()
        .any(|finding| finding.code == "non_authoritative_context_verification_failed")
}

#[test]
fn malformed_or_mismatched_worker_record_fails_closed() {
    let mismatch = serde_json::to_vec(&worker_result("LEASE-N02-WRONG-001")).unwrap();
    for (label, value) in [
        ("malformed", b"{".as_slice()),
        ("mismatch", mismatch.as_slice()),
    ] {
        let repo = exact_repo(&format!("context-v2-worker-{label}"));
        repo.write(RESULT_PATH, value);
        repo.commit();
        let catalog = catalog(&repo).unwrap();
        assert!(evidence_failed(&catalog));
        assert!(!catalog.entries().iter().any(|entry| {
            entry.relative_path == RESULT_PATH && entry.authority_state == AuthorityState::Context
        }));
    }
}

#[test]
fn matching_invalid_worker_directory_entry_fails_closed() {
    let repo = exact_repo("context-v2-worker-extra");
    repo.write(
        &format!("{RESULTS_ROOT}/LEASE-N02-INVALID-001.json"),
        b"not a worker result",
    );
    repo.commit();
    assert!(evidence_failed(&catalog(&repo).unwrap()));
}

#[test]
fn minimal_worker_shape_cannot_self_classify_as_worker_result_v1() {
    let repo = exact_repo("context-v2-worker-minimal-bait");
    repo.write(
        RESULT_PATH,
        br#"{"worker":"/root/bait","lease_id":"LEASE-N02-TEST-CONTEXT-001","no_claim_statement":"This worker does not claim readiness, release, or completion."}"#,
    );
    repo.commit();
    let catalog = catalog(&repo).unwrap();
    assert!(evidence_failed(&catalog));
    assert!(!catalog.entries().iter().any(|entry| {
        entry.relative_path == RESULT_PATH && entry.authority_state == AuthorityState::Context
    }));
}

#[test]
fn undeclared_nonconforming_worker_evidence_fails_closed() {
    let repo = exact_repo("context-v2-worker-undeclared");
    let historical = format!("{RESULTS_ROOT}/LEASE-N03-HISTORICAL-001.json");
    repo.write(&historical, br#"{"legacy":"untrusted"}"#);
    repo.commit();
    let catalog = catalog(&repo).unwrap();
    assert!(evidence_failed(&catalog));
    assert!(!catalog.entries().iter().any(|entry| {
        entry.relative_path == historical && entry.authority_state == AuthorityState::Context
    }));
}

#[test]
fn non_worker_directory_context_does_not_expand_evidence_policy() {
    let repo = exact_repo("context-v2-worker-non-worker-context");
    let note = format!("{RESULTS_ROOT}/README.md");
    repo.write(
        &note,
        b"ordinary documentation remains outside the evidence policy",
    );
    repo.commit();
    let catalog = catalog(&repo).unwrap();
    assert!(!evidence_failed(&catalog));
    assert!(!catalog.entries().iter().any(|entry| {
        entry.relative_path == note && entry.authority_state == AuthorityState::Context
    }));
}

#[test]
fn historical_worker_exception_is_exact_digest_bound() {
    let repo = exact_repo("context-v2-worker-historical-tamper");
    repo.write(
        &format!("{RESULTS_ROOT}/LEASE-N03-CAPTURE-001.json"),
        b"tampered",
    );
    repo.commit();
    assert!(evidence_failed(&catalog(&repo).unwrap()));
}

#[test]
fn historical_worker_exception_must_remain_present() {
    let repo = exact_repo("context-v2-worker-historical-missing");
    fs::remove_file(
        repo.root
            .join(RESULTS_ROOT)
            .join("LEASE-N03-CAPTURE-001.json"),
    )
    .unwrap();
    repo.commit();
    assert!(evidence_failed(&catalog(&repo).unwrap()));
}

#[test]
fn historical_registry_repoint_is_rejected() {
    let repo = exact_repo("context-v2-worker-historical-registry-repoint");
    let mut registry = live_registry();
    registry["evidence_contexts"][0]["historical_files"][0]["sha256"] = json!("0".repeat(64));
    registry["evidence_contexts"][0]["historical_content_set_digest"] = json!("0".repeat(64));
    repo.write(REGISTRY, &serde_json::to_vec(&registry).unwrap());
    repo.commit();
    assert!(
        catalog(&repo)
            .unwrap()
            .findings()
            .iter()
            .any(|finding| { finding.code == "invalid_non_authoritative_context_registry" })
    );
}

#[test]
fn duplicate_key_worker_record_is_rejected_without_secret_echo() {
    let repo = exact_repo("context-v2-worker-duplicate");
    let valid = serde_json::to_string(&worker_result("LEASE-N02-TEST-CONTEXT-001")).unwrap();
    let duplicate = format!(
        "{{\"worker\":\"/root/SECRET_CANARY\",{}",
        valid.strip_prefix('{').unwrap()
    );
    repo.write(RESULT_PATH, duplicate.as_bytes());
    repo.commit();
    let catalog = catalog(&repo).unwrap();
    let finding = catalog
        .findings()
        .iter()
        .find(|finding| finding.code == "non_authoritative_context_verification_failed")
        .unwrap();
    assert!(!finding.message.contains("SECRET_CANARY"));
}

#[cfg(unix)]
#[test]
fn symlinked_worker_record_is_never_context_authority() {
    let repo = exact_repo("context-v2-worker-symlink");
    fs::remove_file(repo.root.join(RESULT_PATH)).unwrap();
    std::os::unix::fs::symlink(
        "../../ultragoal-contract-2026-07/README.md",
        repo.root.join(RESULT_PATH),
    )
    .unwrap();
    repo.commit();
    let catalog = catalog(&repo).unwrap();
    assert!(evidence_failed(&catalog));
    assert!(!catalog.entries().iter().any(|entry| {
        entry.relative_path == RESULT_PATH && entry.authority_state == AuthorityState::Context
    }));
}

#[cfg(unix)]
#[test]
fn symlinked_worker_directory_is_never_context_authority() {
    let repo = exact_repo("context-v2-worker-directory-symlink");
    let outside = repo.root.join("worker-results-outside");
    fs::create_dir(&outside).unwrap();
    fs::write(
        outside.join("LEASE-N02-TEST-CONTEXT-001.json"),
        serde_json::to_vec(&worker_result("LEASE-N02-TEST-CONTEXT-001")).unwrap(),
    )
    .unwrap();
    fs::remove_dir_all(repo.root.join(RESULTS_ROOT)).unwrap();
    std::os::unix::fs::symlink(&outside, repo.root.join(RESULTS_ROOT)).unwrap();
    repo.commit();
    let catalog = catalog(&repo).unwrap();
    assert!(evidence_failed(&catalog));
    assert!(!catalog.entries().iter().any(|entry| {
        entry.relative_path == RESULT_PATH && entry.authority_state == AuthorityState::Context
    }));
}
