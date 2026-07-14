use crate::durable_journal_fixture::*;
use crate::orchestration::*;
use crate::orchestration_fixture::*;

#[test]
fn full_reviewed_flow_requires_root_acceptance_before_completion() {
    let (mut engine, _, _journal) = durable_engine("reviewed-flow");
    engine.grant_lease(1, lease()).unwrap();
    engine.start(2, "lease-001").unwrap();
    let lease = lease();
    let package = package("node-a", &[], "node_a");
    let result = result_for(&lease, &package);
    let review = review_for_result(&result, ReviewDecision::Pass);
    let proposal = propose_acceptance(&binding(), &package, &lease, &result, &review).unwrap();
    engine.submit_structural(3, "lease-001", &result).unwrap();
    engine
        .assign_review(4, "lease-001", worker("reviewer-a"))
        .unwrap();
    engine.record_review(5, "lease-001", review).unwrap();
    let intent = integration_intent_for(&proposal);
    assert_eq!(
        engine.begin_integration(6, intent).unwrap_err(),
        OrchestrationError::InvalidTransition
    );
    engine.accept(7, &proposal).unwrap();
    let workspace = prepare_full_integration(&mut engine, &proposal, 8);
    let integration = integration_for(&proposal, 10);
    let mut incomplete_integration = integration.clone();
    incomplete_integration.applied_changes.clear();
    assert_eq!(
        engine
            .complete(10, workspace.observer(), incomplete_integration)
            .unwrap_err(),
        OrchestrationError::IntegrationAmbiguous
    );
    engine
        .complete(10, workspace.observer(), integration)
        .unwrap();
    assert!(engine.plan().unwrap().ready.is_empty());
}
