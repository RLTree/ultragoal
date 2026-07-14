use super::fixture::*;
use std::collections::BTreeSet;
use std::fs;
use std::thread;
use ultragoal::orchestration::product::command::OrchestrationStateRequest;
use ultragoal::orchestration::product::runtime_adapter::{
    OrchestrationRuntimeAdapter, RuntimeActionRequest, RuntimeActionSource,
};
use ultragoal::orchestration::product::{
    PermitReplayState, ProductError, ProductWorkspace, ResumeRequest,
};

#[test]
fn same_head_clone_execute_refuses_before_reservation_and_origin_remains_usable() {
    let (origin, head) = interrupted_root("execute-origin");
    let clone = clone_journal(&origin, "execute-clone");
    let authority_root = TestRoot::new("execute-authority", 0o700);
    let (action, permit, authority) = issue_resume(&authority_root, &origin, head.clone(), 2);
    let context = context();
    let clone_workspace = ProductWorkspace::open(clone.path()).unwrap();
    let clone_adapter = OrchestrationRuntimeAdapter::new(&context, &clone_workspace).unwrap();
    let clone_view = clone_adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: head.clone(),
            tick: 2,
            live_workers: BTreeSet::new(),
        })
        .unwrap();
    let clone_action = clone_view.state().root_action_requests[0].clone();
    let clone_request = resume_request(&clone_action, 2);
    let authority_before = recursive_fingerprint(authority_root.path());
    let origin_before = recursive_fingerprint(origin.path());
    let clone_before = recursive_fingerprint(clone.path());
    assert_eq!(
        clone_adapter
            .execute_production_action(
                &authority,
                RuntimeActionSource::Current(&clone_view),
                &clone_action,
                &permit,
                &clone_request,
            )
            .unwrap_err(),
        ProductError::AuthorityInvalid
    );
    assert_unchanged(
        &authority_root,
        &origin,
        &clone,
        &authority_before,
        &origin_before,
        &clone_before,
    );
    assert_eq!(
        authority.replay_state(&permit).unwrap(),
        Some(PermitReplayState::Issued)
    );

    let origin_workspace = ProductWorkspace::open(origin.path()).unwrap();
    let origin_adapter = OrchestrationRuntimeAdapter::new(&context, &origin_workspace).unwrap();
    let origin_view = origin_adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: head,
            tick: 2,
            live_workers: BTreeSet::new(),
        })
        .unwrap();
    origin_adapter
        .execute_production_action(
            &authority,
            RuntimeActionSource::Current(&origin_view),
            &action,
            &permit,
            &resume_request(&action, 2),
        )
        .unwrap();
    assert_eq!(
        authority.replay_state(&permit).unwrap(),
        Some(PermitReplayState::Committed)
    );
}

#[test]
fn concurrent_clone_substitution_and_wrong_request_never_reserve_origin() {
    let (origin, head) = interrupted_root("execute-race-origin");
    let clone = clone_journal(&origin, "execute-race-clone");
    let authority_root = TestRoot::new("execute-race-authority", 0o700);
    let (action, permit, authority) = issue_resume(&authority_root, &origin, head.clone(), 2);
    let before = recursive_fingerprint(authority_root.path());
    thread::scope(|scope| {
        let children = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    let context = context();
                    let workspace = ProductWorkspace::open(clone.path()).unwrap();
                    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
                    let view = adapter
                        .inspect_current(&OrchestrationStateRequest {
                            expected_head: head.clone(),
                            tick: 2,
                            live_workers: BTreeSet::new(),
                        })
                        .unwrap();
                    let clone_action = view.state().root_action_requests[0].clone();
                    adapter
                        .execute_production_action(
                            &authority,
                            RuntimeActionSource::Current(&view),
                            &clone_action,
                            &permit,
                            &resume_request(&clone_action, 2),
                        )
                        .unwrap_err()
                })
            })
            .collect::<Vec<_>>();
        for child in children {
            assert_eq!(child.join().unwrap(), ProductError::AuthorityInvalid);
        }
    });
    let context = context();
    let workspace = ProductWorkspace::open(origin.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let view = adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: head,
            tick: 2,
            live_workers: BTreeSet::new(),
        })
        .unwrap();
    assert_eq!(
        adapter
            .execute_production_action(
                &authority,
                RuntimeActionSource::Current(&view),
                &action,
                &permit,
                &resume_request(&action, 3),
            )
            .unwrap_err(),
        ProductError::AuthorityOperationMismatch
    );
    assert_eq!(recursive_fingerprint(authority_root.path()), before);
    assert_eq!(
        authority.replay_state(&permit).unwrap(),
        Some(PermitReplayState::Issued)
    );
    adapter
        .execute_production_action(
            &authority,
            RuntimeActionSource::Current(&view),
            &action,
            &permit,
            &resume_request(&action, 2),
        )
        .unwrap();
    assert_eq!(
        authority.replay_state(&permit).unwrap(),
        Some(PermitReplayState::Committed)
    );
}

#[test]
fn wrong_root_binding_target_permits_and_action_refuse_before_reservation() {
    let (origin, head) = interrupted_root("execute-substitution-origin");
    let authority_root = TestRoot::new("execute-substitution-authority", 0o700);
    let (action, permit, authority) = issue_resume(&authority_root, &origin, head.clone(), 2);
    let context = context();
    let workspace = ProductWorkspace::open(origin.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let view = adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: head,
            tick: 2,
            live_workers: BTreeSet::new(),
        })
        .unwrap();
    let request = resume_request(&action, 2);
    let before = recursive_fingerprint(authority_root.path());
    let mut wrong_action = action.clone();
    wrong_action.workspace_identity = digest('f');
    assert_eq!(
        adapter
            .execute_production_action(
                &authority,
                RuntimeActionSource::Current(&view),
                &wrong_action,
                &permit,
                &request,
            )
            .unwrap_err(),
        ProductError::AuthorityInvalid
    );
    for substituted in direct_execution_substituted_permits(&permit) {
        assert_eq!(
            adapter
                .execute_production_action(
                    &authority,
                    RuntimeActionSource::Current(&view),
                    &action,
                    &substituted,
                    &request,
                )
                .unwrap_err(),
            ProductError::AuthorityInvalid
        );
    }
    assert_eq!(recursive_fingerprint(authority_root.path()), before);
    assert_eq!(
        authority.replay_state(&permit).unwrap(),
        Some(PermitReplayState::Issued)
    );
}

fn resume_request(
    action: &ultragoal::orchestration::product::command::RootActionRequest,
    tick: u64,
) -> RuntimeActionRequest {
    RuntimeActionRequest::Resume(ResumeRequest {
        expected_head: action.expected_head.clone(),
        tick,
        live_workers: BTreeSet::new(),
        target: action.target.clone(),
    })
}

fn clone_journal(origin: &TestRoot, label: &str) -> TestRoot {
    let clone = TestRoot::new(label, 0o700);
    for entry in fs::read_dir(origin.path()).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), clone.path().join(entry.file_name())).unwrap();
    }
    clone
}

fn assert_unchanged(
    authority: &TestRoot,
    origin: &TestRoot,
    clone: &TestRoot,
    authority_before: &[(String, u64, u32, u64, u64, String)],
    origin_before: &[(String, u64, u32, u64, u64, String)],
    clone_before: &[(String, u64, u32, u64, u64, String)],
) {
    assert_eq!(recursive_fingerprint(authority.path()), authority_before);
    assert_eq!(recursive_fingerprint(origin.path()), origin_before);
    assert_eq!(recursive_fingerprint(clone.path()), clone_before);
}
