use super::durable_journal_fixture::*;
use super::orchestration_fixture::*;
use crate::orchestration::*;
use std::collections::BTreeSet;
use std::fs;

fn accepted_with_two_changes(
    label: &str,
) -> (Orchestrator<CountingSink>, AcceptanceProposal, JournalRoot) {
    let (mut engine, _, journal) = durable_engine(label);
    let lease = lease();
    let package = package("node-a", &[], "node_a");
    let mut result = result_for(&lease, &package);
    result.requested_root_changes = vec![
        RootChangeRequest::new(
            "validator/src/lib.rs",
            &content_digest(LIVE_LIB_BYTES),
            Some("module wiring"),
        )
        .unwrap(),
        RootChangeRequest::new(
            "plugin-manifest-draft.json",
            &content_digest(LIVE_MANIFEST_BYTES),
            Some("manifest wiring"),
        )
        .unwrap(),
    ];
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

#[test]
fn none_partial_conflict_and_full_observations_are_distinct() {
    let (mut engine, proposal, _journal) = accepted_with_two_changes("integration-states");
    let intent = integration_intent_for(&proposal);
    let workspace = live_workspace_with_files(&[
        ("validator/src/lib.rs", LIVE_PRIOR_BYTES),
        ("plugin-manifest-draft.json", LIVE_PRIOR_BYTES),
    ]);
    engine.begin_integration(7, intent).unwrap();
    assert_eq!(
        engine.observe_integration(8, workspace.observer()).unwrap(),
        IntegrationDisposition::None
    );
    fs::write(
        workspace.path().join("validator/src/lib.rs"),
        LIVE_LIB_BYTES,
    )
    .unwrap();
    assert_eq!(
        engine.observe_integration(9, workspace.observer()).unwrap(),
        IntegrationDisposition::Partial
    );
    assert_eq!(
        engine
            .recovery_report(&binding(), 9, &BTreeSet::new())
            .integration_disposition,
        Some(IntegrationDisposition::Partial)
    );
    assert_eq!(
        engine
            .complete(10, workspace.observer(), integration_for(&proposal, 10))
            .unwrap_err(),
        OrchestrationError::IntegrationAmbiguous
    );
    fs::write(
        workspace.path().join("plugin-manifest-draft.json"),
        b"foreign bytes\n",
    )
    .unwrap();
    assert_eq!(
        engine
            .observe_integration(10, workspace.observer())
            .unwrap(),
        IntegrationDisposition::Conflict
    );
    fs::write(
        workspace.path().join("plugin-manifest-draft.json"),
        LIVE_MANIFEST_BYTES,
    )
    .unwrap();
    assert_eq!(
        engine
            .observe_integration(11, workspace.observer())
            .unwrap(),
        IntegrationDisposition::Full
    );
    engine
        .complete(12, workspace.observer(), integration_for(&proposal, 12))
        .unwrap();
}

#[test]
fn integration_intent_and_partial_observation_survive_restart() {
    let (mut engine, proposal, journal) = accepted_with_two_changes("integration-restart");
    let intent = integration_intent_for(&proposal);
    let workspace = live_workspace_with_files(&[
        ("validator/src/lib.rs", LIVE_LIB_BYTES),
        ("plugin-manifest-draft.json", LIVE_PRIOR_BYTES),
    ]);
    let intent_id = intent.intent_id().unwrap();
    engine.begin_integration(7, intent.clone()).unwrap();
    assert_eq!(
        engine.observe_integration(8, workspace.observer()).unwrap(),
        IntegrationDisposition::Partial
    );
    let expected_head = engine.journal_head().unwrap().clone();
    drop(engine);

    let (sink, calls) = CountingSink::new();
    let mut restarted = Orchestrator::restart_durable(
        graph_one(),
        policy(),
        expected_head,
        root(),
        journal.path(),
        sink,
    )
    .unwrap();
    let report = restarted.recovery_report(&binding(), 8, &BTreeSet::new());
    assert_eq!(report.pending_integration_id, Some(intent_id));
    assert_eq!(
        report.integration_disposition,
        Some(IntegrationDisposition::Partial)
    );
    assert_eq!(calls.get(), 0);
    fs::write(
        workspace.path().join("plugin-manifest-draft.json"),
        LIVE_MANIFEST_BYTES,
    )
    .unwrap();
    restarted
        .observe_integration(9, workspace.observer())
        .unwrap();
    restarted
        .complete(10, workspace.observer(), integration_for(&proposal, 10))
        .unwrap();
}

#[test]
fn ambiguous_effect_blocks_root_integration_until_reconciled() {
    let package_a = package("node-a", &[], "node_a");
    let package_b = package("node-b", &[], "node_b");
    let graph = WorkGraph::derive(vec![package_a.clone(), package_b]).unwrap();
    let journal = JournalRoot::new("integration-effect-block");
    let (sink, _) = CountingSink::new();
    let mut engine = Orchestrator::new_durable(
        graph,
        policy(),
        binding(),
        root(),
        bootstrap(),
        journal.path(),
        sink,
    )
    .unwrap();
    let lease_a = lease();
    let mut lease_b = lease_with_scope("lease-002", "node-b", "worker-b", scope("node_b"));
    lease_b.issued_tick = 2;
    let result = result_for(&lease_a, &package_a);
    let review = review_for_result(&result, ReviewDecision::Pass);
    let proposal = propose_acceptance(&binding(), &package_a, &lease_a, &result, &review).unwrap();
    engine.grant_lease(1, lease_a).unwrap();
    engine.grant_lease(2, lease_b).unwrap();
    engine.start(3, "lease-001").unwrap();
    engine.start(4, "lease-002").unwrap();
    engine.submit_structural(5, "lease-001", &result).unwrap();
    engine
        .assign_review(6, "lease-001", worker("reviewer-a"))
        .unwrap();
    engine.record_review(7, "lease-001", review).unwrap();
    engine.accept(8, &proposal).unwrap();
    let request = EffectRequest {
        lease_id: "lease-002".to_owned(),
        binding: binding(),
        effect: effect(EffectClass::WorkspaceWrite, "leased-source/node_b"),
        operation_id: "operation-pending".to_owned(),
        payload_digest: digest('9'),
    };
    let event = EventKind::EffectIntent {
        lease_id: request.lease_id.clone(),
        request,
    };
    engine.append(worker("worker-b"), 9, event).unwrap();
    assert_eq!(
        engine
            .begin_integration(10, integration_intent_for(&proposal))
            .unwrap_err(),
        OrchestrationError::IntegrationAmbiguous
    );
    engine
        .reconcile_effect(
            10,
            "lease-002",
            EffectResolution {
                operation_id: "operation-pending".to_owned(),
                evidence_digest: digest('8'),
                outcome: EffectOutcome::NotApplied,
            },
        )
        .unwrap();
    engine
        .begin_integration(11, integration_intent_for(&proposal))
        .unwrap();
}
