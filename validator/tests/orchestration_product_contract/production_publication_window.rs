use super::product_fixture::*;
use crate::orchestration::product::command::InterruptedRecoveryRequest;
use crate::orchestration::product::runtime_adapter::{
    OrchestrationRuntimeAdapter, RuntimeActionOutcome, RuntimeActionRequest, RuntimeActionSource,
};
use crate::orchestration::product::{ProductWorkspace, ProductionRootAuthority, RecoverRequest};
use crate::orchestration::*;
use std::collections::BTreeSet;
use std::fs;

#[test]
fn production_recovery_returns_the_committed_identity_after_root_substitution() {
    let (root, mut engine) = durable_engine("production-publication-window", false);
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
    let mut log = fs::read(root.path().join("events.jsonl")).unwrap();
    log.extend(journal_frame_bytes(&event));
    log.push(b'\n');
    fs::write(root.path().join("events.jsonl"), log).unwrap();

    let context = context();
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let inspection = InterruptedRecoveryRequest {
        expected_prior_head: prior.clone(),
        expected_event_id: event.event_id.clone(),
        recovered_binding: binding(),
        tick: 3,
        live_workers: BTreeSet::from(["worker-a".to_owned()]),
    };
    let view = adapter.inspect_interrupted(&inspection).unwrap();
    let action = view.preview().root_action_request.clone();
    let authority_root = TestRoot::new("production-publication-window-authority");
    let authority =
        ProductionRootAuthority::open_or_initialize(authority_root.path(), root_actor()).unwrap();
    let permit = adapter
        .issue_production_action(
            &authority,
            RuntimeActionSource::Interrupted(&view),
            &action,
            13,
        )
        .unwrap();

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

    let outcome = adapter
        .execute_production_action(
            &authority,
            RuntimeActionSource::Interrupted(&view),
            &action,
            &permit,
            &RuntimeActionRequest::Recover(RecoverRequest {
                expected_prior_head: prior,
                expected_event_id: event.event_id,
                recovered_binding: binding(),
                tick: 3,
                live_workers: BTreeSet::from(["worker-a".to_owned()]),
                target: action.target.clone(),
            }),
        )
        .unwrap();
    let RuntimeActionOutcome::Recover(recovered) = outcome else {
        panic!("recovery outcome required");
    };
    assert!(!recovered.workspace_path_current_after_commit);
    assert_eq!(recovered.workspace_identity, workspace.identity());
    assert_eq!(
        FileJournal::open(&moved).unwrap().inspect().unwrap().head,
        recovered.recovered_head
    );
    assert_eq!(
        FileJournal::open(&root_path).unwrap_err(),
        OrchestrationError::JournalCorrupt
    );

    fs::remove_dir_all(&root_path).unwrap();
    fs::rename(moved, root_path).unwrap();
}
