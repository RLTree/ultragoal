use crate::orchestration::*;
use crate::orchestration_product::*;
use crate::product_fixture::*;
use std::collections::BTreeSet;
use std::fs;
use std::sync::{Arc, Barrier};

fn interrupted_heartbeat(label: &str) -> (TestRoot, JournalHead, OrchestrationEvent) {
    let (root, mut engine) = durable_engine(label, false);
    engine.grant_lease(1, lease(40)).unwrap();
    engine.start(2, "lease-001").unwrap();
    let prior = engine.journal_head().unwrap().clone();
    let event = engine
        .event_log()
        .next(
            &binding(),
            worker_actor(),
            3,
            EventKind::Heartbeat {
                lease_id: "lease-001".to_owned(),
            },
        )
        .unwrap();
    drop(engine);
    let mut bytes = fs::read(root.path().join("events.jsonl")).unwrap();
    bytes.extend(journal_frame_bytes(&event));
    bytes.push(b'\n');
    fs::write(root.path().join("events.jsonl"), bytes).unwrap();
    (root, prior, event)
}

fn recovery_subject(
    root: &TestRoot,
    prior: &JournalHead,
    event: &OrchestrationEvent,
    tick: u64,
    live_workers: BTreeSet<String>,
) -> (ProductWorkspace, RootAuthority, RootPermit, RecoverRequest) {
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let authority = authority();
    let target = PermitTarget {
        lease_id: Some("lease-001".to_owned()),
        operation_id: Some(event.event_id.clone()),
        recovered_binding: Some(binding()),
        ..PermitTarget::default()
    };
    let permit = permit(
        &authority,
        RootOperation::Recover,
        &workspace,
        prior,
        tick,
        target.clone(),
    );
    let request = RecoverRequest {
        expected_prior_head: prior.clone(),
        expected_event_id: event.event_id.clone(),
        recovered_binding: binding(),
        tick,
        live_workers,
        target,
    };
    (workspace, authority, permit, request)
}

#[test]
fn interrupted_publication_recovers_only_the_exact_authenticated_event() {
    let (root, prior, event) = interrupted_heartbeat("interrupted-publication");
    let (workspace, authority, permit, request) = recovery_subject(
        &root,
        &prior,
        &event,
        3,
        BTreeSet::from(["worker-a".to_owned()]),
    );
    let outcome = recover(&context(), &workspace, &authority, &permit, &request).unwrap();
    assert!(outcome.workspace_path_current_after_commit);
    assert_eq!(outcome.workspace_identity, workspace.identity());
    assert_eq!(outcome.recovered_head.event_count, prior.event_count + 1);
    assert_eq!(outcome.snapshot.recovery.resumable_leases, ["lease-001"]);
}

#[test]
fn orphaned_or_expired_recovery_refuses_before_publishing_the_head() {
    for (label, tick, live_workers, expected) in [
        (
            "orphaned-recovery",
            3,
            BTreeSet::new(),
            ProductError::UnknownWorker,
        ),
        (
            "expired-recovery",
            41,
            BTreeSet::from(["worker-a".to_owned()]),
            ProductError::LeaseExpired,
        ),
    ] {
        let (root, prior, event) = interrupted_heartbeat(label);
        let (workspace, authority, permit, request) =
            recovery_subject(&root, &prior, &event, tick, live_workers);
        let before = recursive_fingerprint(root.path());
        assert_eq!(
            recover(&context(), &workspace, &authority, &permit, &request).unwrap_err(),
            expected
        );
        assert_eq!(recursive_fingerprint(root.path()), before);
        assert_eq!(
            FileJournal::open(root.path()).unwrap_err(),
            OrchestrationError::JournalCorrupt
        );
    }
}

#[test]
fn concurrent_recovery_publishes_one_exact_head() {
    let (root, prior, event) = interrupted_heartbeat("concurrent-recovery");
    let (workspace, authority, permit, request) = recovery_subject(
        &root,
        &prior,
        &event,
        3,
        BTreeSet::from(["worker-a".to_owned()]),
    );
    let barrier = Arc::new(Barrier::new(2));
    let handles = (0..2)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            let context = context();
            let workspace = workspace.clone();
            let authority = authority.clone();
            let permit = permit.clone();
            let request = request.clone();
            std::thread::spawn(move || {
                barrier.wait();
                recover(&context, &workspace, &authority, &permit, &request)
            })
        })
        .collect::<Vec<_>>();
    let results = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(ProductError::ConcurrentUpdate)))
            .count(),
        1
    );
    let recovered = results.into_iter().find_map(Result::ok).unwrap();
    let committed = FileJournal::open(root.path()).unwrap().inspect().unwrap();
    assert_eq!(committed.head, recovered.recovered_head);
    assert_eq!(committed.head.event_count, prior.event_count + 1);
}

#[test]
fn publication_window_root_substitution_returns_committed_identity() {
    let (root, prior, event) = interrupted_heartbeat("publication-window-substitution");
    let (workspace, authority, permit, request) = recovery_subject(
        &root,
        &prior,
        &event,
        3,
        BTreeSet::from(["worker-a".to_owned()]),
    );
    let expected_identity = workspace.identity().to_owned();
    let root_path = root.path().to_path_buf();
    let moved = root.path().with_extension("permit-bound");
    let hook_root = root_path.clone();
    let hook_moved = moved.clone();
    FileJournal::set_test_pre_publication_hook(move || {
        fs::rename(&hook_root, &hook_moved).unwrap();
        fs::create_dir(&hook_root).unwrap();
        for name in ["events.jsonl", "head.json", "journal.lock"] {
            fs::copy(hook_moved.join(name), hook_root.join(name)).unwrap();
        }
    });

    let outcome = recover(&context(), &workspace, &authority, &permit, &request).unwrap();
    assert_eq!(outcome.workspace_identity, expected_identity);
    assert!(!outcome.workspace_path_current_after_commit);
    let committed = FileJournal::open(&moved).unwrap().inspect().unwrap();
    assert_eq!(committed.head, outcome.recovered_head);
    assert_eq!(
        FileJournal::open(&root_path).unwrap_err(),
        OrchestrationError::JournalCorrupt
    );

    fs::remove_dir_all(&root_path).unwrap();
    fs::rename(moved, root_path).unwrap();
}
