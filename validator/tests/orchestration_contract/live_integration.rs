use super::durable_journal_fixture::*;
use super::orchestration_fixture::*;
use crate::orchestration::*;
use std::fs;

fn accepted_engine(label: &str) -> (Orchestrator<CountingSink>, AcceptanceProposal, JournalRoot) {
    let (mut engine, _, journal) = durable_engine(label);
    let lease = lease();
    let package = package("node-a", &[], "node_a");
    let result = result_for(&lease, &package);
    let review = review_for_result(&result, ReviewDecision::Pass);
    let proposal = propose_acceptance(&binding(), &package, &lease, &result, &review).unwrap();
    engine.grant_lease(1, lease).unwrap();
    engine.start(2, "lease-001").unwrap();
    engine.submit_structural(3, "lease-001", &result).unwrap();
    engine
        .assign_review(4, "lease-001", worker("reviewer-a"))
        .unwrap();
    engine.record_review(5, "lease-001", review).unwrap();
    engine.accept(6, &proposal).unwrap();
    (engine, proposal, journal)
}

fn journal_bytes(root: &JournalRoot) -> (Vec<u8>, Vec<u8>) {
    (
        fs::read(root.path().join("events.jsonl")).unwrap(),
        fs::read(root.path().join("head.json")).unwrap(),
    )
}

#[test]
fn completion_reads_the_live_workspace_at_the_final_boundary() {
    let (mut engine, proposal, _journal) = accepted_engine("live-completion");
    let intent = integration_intent_for(&proposal);
    let workspace = live_workspace_for_intent(&intent);
    engine.begin_integration(7, intent).unwrap();
    engine
        .complete(8, workspace.observer(), integration_for(&proposal, 8))
        .unwrap();
    assert_ne!(engine.binding(), &binding());
    assert!(engine.plan().unwrap().ready.is_empty());
}

#[test]
fn serialized_expected_digest_synthesis_cannot_cross_the_live_append_boundary() {
    let (mut engine, proposal, journal) = accepted_engine("synthetic-full-denied");
    let intent = integration_intent_for(&proposal);
    let workspace = live_workspace_for_intent(&intent);
    engine.begin_integration(7, intent.clone()).unwrap();
    let live = workspace.observer().observe(&intent).unwrap();
    let synthetic: RootIntegrationObservation =
        serde_json::from_value(serde_json::to_value(live).unwrap()).unwrap();
    let event = EventKind::CandidateRebound {
        observation: synthetic,
        integration: integration_for(&proposal, 8),
    };
    let before_log = engine.event_log();
    let before_files = journal_bytes(&journal);
    assert_eq!(
        engine.append(root(), 8, event).unwrap_err(),
        OrchestrationError::IntegrationAmbiguous
    );
    assert_eq!(engine.event_log(), before_log);
    assert_eq!(journal_bytes(&journal), before_files);
    engine
        .complete(8, workspace.observer(), integration_for(&proposal, 8))
        .unwrap();
    let final_binding = engine.binding().clone();
    let serialized = serde_json::to_vec(&engine.event_log()).unwrap();
    let caller_log: EventLog = serde_json::from_slice(&serialized).unwrap();
    let (sink, _) = CountingSink::new();
    assert_eq!(
        Orchestrator::restart(
            graph_one(),
            policy(),
            final_binding,
            root(),
            caller_log,
            sink,
        )
        .err()
        .unwrap(),
        OrchestrationError::IntegrationAmbiguous
    );
}

#[test]
fn mutation_after_status_observation_is_rejected_by_final_live_revalidation() {
    let (mut engine, proposal, journal) = accepted_engine("live-final-mutation");
    let intent = integration_intent_for(&proposal);
    let workspace = live_workspace_for_intent(&intent);
    engine.begin_integration(7, intent).unwrap();
    assert_eq!(
        engine.observe_integration(8, workspace.observer()).unwrap(),
        IntegrationDisposition::Full
    );
    fs::write(
        workspace.path().join("validator/src/lib.rs"),
        b"substituted after observation\n",
    )
    .unwrap();
    let before_log = engine.event_log();
    let before_files = journal_bytes(&journal);
    assert_eq!(
        engine
            .complete(9, workspace.observer(), integration_for(&proposal, 9))
            .unwrap_err(),
        OrchestrationError::IntegrationAmbiguous
    );
    assert_eq!(engine.event_log(), before_log);
    assert_eq!(journal_bytes(&journal), before_files);
    fs::write(
        workspace.path().join("validator/src/lib.rs"),
        LIVE_LIB_BYTES,
    )
    .unwrap();
    engine
        .complete(9, workspace.observer(), integration_for(&proposal, 9))
        .unwrap();
}
