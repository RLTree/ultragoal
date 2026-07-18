use super::fixture::*;
use std::collections::BTreeSet;
use ultragoal::orchestration::product::command::OrchestrationStateRequest;
use ultragoal::orchestration::product::runtime_adapter::{
    OrchestrationRuntimeAdapter, RuntimeActionOutcome, RuntimeActionRequest, RuntimeActionSource,
};
use ultragoal::orchestration::product::{
    PermitReplayState, ProductError, ProductWorkspace, ProductionRootAuthority, ResumeRequest,
};

#[test]
fn submitted_result_resume_commits_only_the_observed_lease_and_commitment() {
    let (journal, _artifacts, head, commitment) = submitted_interrupted("submitted-resume");
    let authority_root = TestRoot::new("submitted-resume-authority", 0o700);
    let context = context();
    let workspace = ProductWorkspace::open(journal.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let view = adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: head.clone(),
            tick: 5,
            live_workers: BTreeSet::from(["worker-a".to_owned()]),
        })
        .unwrap();
    let action = view.state().root_action_requests[0].clone();
    assert_eq!(action.target.lease_id.as_deref(), Some("lease-001"));
    assert_eq!(
        action.target.result_commitment_id.as_deref(),
        Some(commitment.as_str())
    );
    let authority =
        ProductionRootAuthority::open_or_initialize(authority_root.path(), root_actor()).unwrap();
    let permit = adapter
        .issue_production_action(&authority, RuntimeActionSource::Current(&view), &action, 15)
        .unwrap();
    let outcome = adapter
        .execute_production_action(
            &authority,
            RuntimeActionSource::Current(&view),
            &action,
            &permit,
            &RuntimeActionRequest::Resume(ResumeRequest {
                expected_head: head.clone(),
                tick: 5,
                live_workers: BTreeSet::from(["worker-a".to_owned()]),
                target: action.target.clone(),
            }),
        )
        .unwrap();
    let RuntimeActionOutcome::Resume(resumed) = outcome else {
        panic!("resume outcome required");
    };
    assert!(resumed.root_recovered);
    assert_eq!(resumed.current_head.event_count, head.event_count + 1);
    assert_eq!(
        resumed.snapshot.commitments["lease-001"].result_commitment_id,
        commitment
    );
    assert_eq!(
        authority.replay_state(&permit).unwrap(),
        Some(PermitReplayState::Committed)
    );
}

#[test]
fn submitted_result_and_authority_expiry_refuse_without_reserving() {
    let (journal, _artifacts, head, _) = submitted_interrupted("submitted-resume-refusal");
    let authority_root = TestRoot::new("submitted-resume-refusal-authority", 0o700);
    let context = context();
    let workspace = ProductWorkspace::open(journal.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let view = adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: head.clone(),
            tick: 5,
            live_workers: BTreeSet::from(["worker-a".to_owned()]),
        })
        .unwrap();
    let action = view.state().root_action_requests[0].clone();
    let authority =
        ProductionRootAuthority::open_or_initialize(authority_root.path(), root_actor()).unwrap();
    let permit = adapter
        .issue_production_action(&authority, RuntimeActionSource::Current(&view), &action, 15)
        .unwrap();
    let before_journal = recursive_fingerprint(journal.path());
    let before_authority = recursive_fingerprint(authority_root.path());
    let mut wrong_target = action.target.clone();
    wrong_target.result_commitment_id = Some(digest('f'));
    assert_eq!(
        adapter
            .execute_production_action(
                &authority,
                RuntimeActionSource::Current(&view),
                &action,
                &permit,
                &RuntimeActionRequest::Resume(ResumeRequest {
                    expected_head: head.clone(),
                    tick: 5,
                    live_workers: BTreeSet::from(["worker-a".to_owned()]),
                    target: wrong_target,
                }),
            )
            .unwrap_err(),
        ProductError::AuthorityOperationMismatch
    );
    assert_eq!(recursive_fingerprint(journal.path()), before_journal);
    assert_eq!(
        recursive_fingerprint(authority_root.path()),
        before_authority
    );
    assert_eq!(
        authority.replay_state(&permit).unwrap(),
        Some(PermitReplayState::Issued)
    );

    let (expired_journal, _artifacts, expired_head, _) =
        submitted_interrupted("submitted-resume-authority-expired");
    let expired_authority_root = TestRoot::new("submitted-resume-authority-expired-root", 0o700);
    let expired_workspace = ProductWorkspace::open(expired_journal.path()).unwrap();
    let expired_adapter = OrchestrationRuntimeAdapter::new(&context, &expired_workspace).unwrap();
    let expired_view = expired_adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: expired_head.clone(),
            tick: 5,
            live_workers: BTreeSet::from(["worker-a".to_owned()]),
        })
        .unwrap();
    let expired_action = expired_view.state().root_action_requests[0].clone();
    let expired_authority =
        ProductionRootAuthority::open_or_initialize(expired_authority_root.path(), root_actor())
            .unwrap();
    let expired_permit = expired_adapter
        .issue_production_action(
            &expired_authority,
            RuntimeActionSource::Current(&expired_view),
            &expired_action,
            5,
        )
        .unwrap();
    let execution_view = expired_adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: expired_head.clone(),
            tick: 6,
            live_workers: BTreeSet::from(["worker-a".to_owned()]),
        })
        .unwrap();
    assert_eq!(
        expired_adapter
            .execute_production_action(
                &expired_authority,
                RuntimeActionSource::Current(&execution_view),
                &expired_action,
                &expired_permit,
                &RuntimeActionRequest::Resume(ResumeRequest {
                    expected_head: expired_head,
                    tick: 6,
                    live_workers: BTreeSet::from(["worker-a".to_owned()]),
                    target: expired_action.target.clone(),
                }),
            )
            .unwrap_err(),
        ProductError::AuthorityExpired
    );
    assert_eq!(
        expired_authority.replay_state(&expired_permit).unwrap(),
        Some(PermitReplayState::Issued)
    );
}

#[test]
fn stale_context_and_expired_submitted_lease_refuse_before_issuance() {
    let (journal, _artifacts, head, _) = submitted_interrupted("submitted-resume-stale-context");
    let workspace = ProductWorkspace::open(journal.path()).unwrap();
    let before = recursive_fingerprint(journal.path());
    let stale = ultragoal::orchestration::product::ProductContext::new(
        super::fixture::graph(),
        super::fixture::policy(),
        ultragoal::orchestration::Binding::new(&digest('c'), &digest('d')).unwrap(),
        root_actor(),
    );
    assert_eq!(
        OrchestrationRuntimeAdapter::new(&stale, &workspace)
            .unwrap()
            .inspect_current(&OrchestrationStateRequest {
                expected_head: head.clone(),
                tick: 5,
                live_workers: BTreeSet::from(["worker-a".to_owned()]),
            })
            .unwrap_err(),
        ProductError::StaleCandidate
    );
    assert_eq!(recursive_fingerprint(journal.path()), before);

    let context = context();
    let expired_view = OrchestrationRuntimeAdapter::new(&context, &workspace)
        .unwrap()
        .inspect_current(&OrchestrationStateRequest {
            expected_head: head,
            tick: 41,
            live_workers: BTreeSet::from(["worker-a".to_owned()]),
        })
        .unwrap();
    assert_eq!(
        expired_view.state().snapshot.recovery.expired_leases,
        ["lease-001"]
    );
    assert!(expired_view.state().root_action_requests.is_empty());
    assert_eq!(recursive_fingerprint(journal.path()), before);
}
