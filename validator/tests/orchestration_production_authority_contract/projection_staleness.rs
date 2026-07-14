use super::fixture::*;
use std::collections::BTreeSet;
use ultragoal::orchestration::product::command::OrchestrationStateRequest;
use ultragoal::orchestration::product::runtime_adapter::{
    OrchestrationRuntimeAdapter, RuntimeActionRequest, RuntimeActionSource,
};
use ultragoal::orchestration::product::{
    PermitReplayState, ProductError, ProductWorkspace, ResumeRequest,
};

#[test]
fn current_and_interrupted_projections_are_recursive_zero_write() {
    let (current, head) = interrupted_root("projection-current");
    let before = recursive_fingerprint(current.path());
    let context = context();
    let workspace = ProductWorkspace::open(current.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: head,
            tick: 2,
            live_workers: BTreeSet::new(),
        })
        .unwrap();
    assert_eq!(recursive_fingerprint(current.path()), before);

    let (interrupted, _, _, request) = interrupted_heartbeat("projection-interrupted");
    let before = recursive_fingerprint(interrupted.path());
    let workspace = ProductWorkspace::open(interrupted.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    adapter.inspect_interrupted(&request).unwrap();
    assert_eq!(recursive_fingerprint(interrupted.path()), before);
}

#[test]
fn stale_view_refuses_before_reservation_and_leaves_permit_usable() {
    let (journal, head) = interrupted_root("stale-view-journal");
    let authority_root = TestRoot::new("stale-view-authority", 0o700);
    let (action, permit, authority) = issue_resume(&authority_root, &journal, head.clone(), 2);
    let context = context();
    let workspace = ProductWorkspace::open(journal.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let view = adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: head.clone(),
            tick: 2,
            live_workers: BTreeSet::new(),
        })
        .unwrap();
    advance_interrupted_root(&journal, head, 2);
    let journal_after_advance = recursive_fingerprint(journal.path());
    let authority_before = recursive_fingerprint(authority_root.path());
    let request = RuntimeActionRequest::Resume(ResumeRequest {
        expected_head: action.expected_head.clone(),
        tick: 2,
        live_workers: BTreeSet::new(),
        target: action.target.clone(),
    });
    assert_eq!(
        adapter
            .execute_production_action(
                &authority,
                RuntimeActionSource::Current(&view),
                &action,
                &permit,
                &request,
            )
            .unwrap_err(),
        ProductError::ConcurrentUpdate
    );
    assert_eq!(recursive_fingerprint(journal.path()), journal_after_advance);
    assert_eq!(
        recursive_fingerprint(authority_root.path()),
        authority_before
    );
    assert_eq!(
        authority.replay_state(&permit).unwrap(),
        Some(PermitReplayState::Issued)
    );
}
