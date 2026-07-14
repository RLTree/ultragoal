use super::fixture::*;
use std::collections::BTreeSet;
use ultragoal::orchestration::product::command::OrchestrationStateRequest;
use ultragoal::orchestration::product::runtime_adapter::{
    OrchestrationRuntimeAdapter, RuntimeActionRequest, RuntimeActionSource,
};
use ultragoal::orchestration::product::{PermitReplayState, ProductWorkspace, ResumeRequest};

#[test]
fn one_transaction_cannot_commit_a_while_executing_b() {
    let (journal_a, head_a) = interrupted_root("transaction-binding-a-journal");
    let (journal_b, head_b) = interrupted_root("transaction-binding-b-journal");
    let root_a = TestRoot::new("transaction-binding-a-authority", 0o700);
    let root_b = TestRoot::new("transaction-binding-b-authority", 0o700);
    let (action_a, permit_a, authority_a) = issue_resume(&root_a, &journal_a, head_a.clone(), 2);
    let (action_b, permit_b, authority_b) = issue_resume(&root_b, &journal_b, head_b.clone(), 2);

    execute_resume(&journal_b, head_b, &action_b, &permit_b, &authority_b);

    assert_eq!(
        authority_a.replay_state(&permit_a).unwrap(),
        Some(PermitReplayState::Issued)
    );
    assert_eq!(
        authority_b.replay_state(&permit_b).unwrap(),
        Some(PermitReplayState::Committed)
    );

    execute_resume(&journal_a, head_a, &action_a, &permit_a, &authority_a);
    assert_eq!(
        authority_a.replay_state(&permit_a).unwrap(),
        Some(PermitReplayState::Committed)
    );
}

fn execute_resume(
    journal: &TestRoot,
    head: ultragoal::orchestration::JournalHead,
    action: &ultragoal::orchestration::product::command::RootActionRequest,
    permit: &ultragoal::orchestration::product::RootPermit,
    authority: &ultragoal::orchestration::product::ProductionRootAuthority,
) {
    let context = context();
    let workspace = ProductWorkspace::open(journal.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let view = adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: head,
            tick: 2,
            live_workers: BTreeSet::new(),
        })
        .unwrap();
    let request = RuntimeActionRequest::Resume(ResumeRequest {
        expected_head: action.expected_head.clone(),
        tick: 2,
        live_workers: BTreeSet::new(),
        target: action.target.clone(),
    });
    adapter
        .execute_production_action(
            authority,
            RuntimeActionSource::Current(&view),
            action,
            permit,
            &request,
        )
        .unwrap();
}
