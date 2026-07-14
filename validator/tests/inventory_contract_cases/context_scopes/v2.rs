use super::{CANDIDATE_ROOT, REGISTRY, catalog, copy_candidate};
use crate::inventory::{ActiveStatus, AuthorityState};
use crate::repository_fixture::{TestRepo, live_root};
use serde_json::{Value, json};
use std::fs;

#[path = "v2/evidence.rs"]
mod evidence;
#[path = "v2/proposal_context.rs"]
mod proposal_context;

const PREDECESSOR_ROOT: &str = "docs/ultragoal-contract-2026-07";
const RESULTS_ROOT: &str = "docs/ultragoal-successor-live/worker-results";
const RESULT_PATH: &str =
    "docs/ultragoal-successor-live/worker-results/LEASE-N02-TEST-CONTEXT-001.json";
const HISTORICAL_RESULTS: [&str; 5] = [
    "LEASE-N03-CAPTURE-001.json",
    "LEASE-N03-CAPTURE-002.json",
    "LEASE-N03-CAPTURE-003.json",
    "LEASE-N03-STATE-001.json",
    "LEASE-N03-STATE-002.json",
];

fn copy_predecessor(repo: &TestRepo) {
    let source = live_root().join(PREDECESSOR_ROOT);
    for entry in walkdir::WalkDir::new(&source)
        .follow_links(false)
        .into_iter()
        .map(Result::unwrap)
        .filter(|entry| entry.file_type().is_file())
    {
        let relative = entry.path().strip_prefix(&source).unwrap();
        repo.write(
            &format!("{PREDECESSOR_ROOT}/{}", relative.to_string_lossy()),
            &fs::read(entry.path()).unwrap(),
        );
    }
}

fn live_registry() -> Value {
    serde_json::from_slice(&fs::read(live_root().join(REGISTRY)).unwrap()).unwrap()
}

fn worker_result(lease_id: &str) -> Value {
    json!({
        "worker": "/root/test-worker",
        "lease_id": lease_id,
        "context_id": "sha256:test-context",
        "candidate_identity": {},
        "base_state": {},
        "final_state": {},
        "touched_paths": [],
        "touched_semantics": [],
        "generated_outputs": [],
        "fixtures": [],
        "effects": [],
        "requirements": [],
        "dependency_nodes": [],
        "changes": [],
        "commands_and_tests": [],
        "artifacts": [],
        "findings": [],
        "unresolved_dependencies": [],
        "requested_root_changes": [],
        "limitations": [],
        "no_claim_statement": "This worker does not claim readiness, release, or completion."
    })
}

fn copy_historical_evidence(repo: &TestRepo) {
    for name in HISTORICAL_RESULTS {
        repo.write(
            &format!("{RESULTS_ROOT}/{name}"),
            &fs::read(live_root().join(RESULTS_ROOT).join(name)).unwrap(),
        );
    }
}

fn exact_repo(label: &str) -> TestRepo {
    let repo = TestRepo::new(label);
    copy_candidate(&repo);
    copy_predecessor(&repo);
    copy_historical_evidence(&repo);
    repo.write(
        "REPORT.md",
        &fs::read(live_root().join("REPORT.md")).unwrap(),
    );
    repo.write(
        RESULT_PATH,
        &serde_json::to_vec(&worker_result("LEASE-N02-TEST-CONTEXT-001")).unwrap(),
    );
    repo.write(REGISTRY, &serde_json::to_vec(&live_registry()).unwrap());
    repo
}

#[test]
fn exact_predecessor_and_worker_evidence_are_context_only() {
    let repo = exact_repo("context-v2-exact");
    repo.commit();
    let catalog = catalog(&repo).unwrap();
    let context = catalog
        .entries()
        .iter()
        .filter(|entry| {
            entry
                .relative_path
                .starts_with(&format!("{CANDIDATE_ROOT}/"))
                || entry
                    .relative_path
                    .starts_with(&format!("{PREDECESSOR_ROOT}/"))
                || entry.relative_path.starts_with(&format!("{RESULTS_ROOT}/"))
        })
        .collect::<Vec<_>>();
    assert_eq!(context.len(), 40);
    assert!(context.iter().all(|entry| {
        entry.authority_state == AuthorityState::Context
            && entry.active_status == ActiveStatus::ContextOnly
            && entry.input_provenance.contains(&REGISTRY.to_owned())
    }));
    assert!(catalog.entries().iter().any(|entry| {
        entry.relative_path == RESULT_PATH && entry.kind == "run-scoped-worker-evidence-context"
    }));
}

#[test]
fn predecessor_tamper_fails_closed_without_partial_demotion() {
    let repo = exact_repo("context-v2-predecessor-tamper");
    repo.write(&format!("{PREDECESSOR_ROOT}/README.md"), b"tampered");
    repo.commit();
    let catalog = catalog(&repo).unwrap();
    assert!(
        catalog
            .findings()
            .iter()
            .any(|finding| { finding.code == "non_authoritative_context_verification_failed" })
    );
    assert!(catalog.entries().iter().any(|entry| {
        entry.relative_path == format!("{PREDECESSOR_ROOT}/README.md")
            && entry.authority_state == AuthorityState::Legacy
    }));
}

#[test]
fn registry_digest_repoint_cannot_reclassify_predecessor_bytes() {
    let repo = exact_repo("context-v2-registry-repoint");
    let mut registry = live_registry();
    registry["exact_contexts"][0]["files"][0]["sha256"] = json!("0".repeat(64));
    repo.write(REGISTRY, &serde_json::to_vec(&registry).unwrap());
    repo.commit();
    let catalog = catalog(&repo).unwrap();
    assert!(
        catalog
            .findings()
            .iter()
            .any(|finding| finding.code == "invalid_non_authoritative_context_registry")
    );
    assert!(catalog.entries().iter().any(|entry| {
        entry.relative_path == format!("{PREDECESSOR_ROOT}/README.md")
            && entry.authority_state == AuthorityState::Legacy
    }));
}

#[test]
fn empty_predecessor_directory_is_excess_coverage() {
    let repo = exact_repo("context-v2-predecessor-empty-dir");
    repo.commit();
    fs::create_dir(repo.root.join(PREDECESSOR_ROOT).join("UNLISTED-EMPTY")).unwrap();
    let catalog = catalog(&repo).unwrap();
    assert!(
        catalog
            .findings()
            .iter()
            .any(|finding| finding.code == "non_authoritative_context_verification_failed")
    );
}
