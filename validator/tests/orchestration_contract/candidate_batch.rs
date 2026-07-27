use super::durable_journal_fixture::*;
use super::orchestration_fixture::*;
use crate::orchestration::*;

fn accepted_batch(
    second_root_path: &str,
) -> (
    Orchestrator<CountingSink>,
    AcceptanceProposal,
    AcceptanceProposal,
    JournalRoot,
) {
    let package_a = package("node-a", &[], "node_a");
    let package_b = package("node-b", &[], "node_b");
    let graph = WorkGraph::derive(vec![package_a.clone(), package_b.clone()]).unwrap();
    let journal = JournalRoot::new("accepted-batch");
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
    let result_a = result_for(&lease_a, &package_a);
    let mut result_b = result_for(&lease_b, &package_b);
    result_b.requested_root_changes = vec![
        RootChangeRequest::new(
            second_root_path,
            &content_digest(LIVE_MANIFEST_BYTES),
            Some("root manifest wiring"),
        )
        .unwrap(),
    ];
    engine.grant_lease(1, lease_a.clone()).unwrap();
    engine.grant_lease(2, lease_b.clone()).unwrap();
    engine.start(3, "lease-001").unwrap();
    engine.start(4, "lease-002").unwrap();
    engine.submit_structural(5, "lease-001", &result_a).unwrap();
    engine.submit_structural(6, "lease-002", &result_b).unwrap();
    engine
        .assign_review(7, "lease-001", worker("reviewer-a"))
        .unwrap();
    engine
        .assign_review(8, "lease-002", worker("reviewer-a"))
        .unwrap();
    let review_a = review_for_result(&result_a, ReviewDecision::Pass);
    let review_b = review_for_result(&result_b, ReviewDecision::Pass);
    engine
        .record_review(9, "lease-001", review_a.clone())
        .unwrap();
    engine
        .record_review(10, "lease-002", review_b.clone())
        .unwrap();
    let proposal_a =
        propose_acceptance(&binding(), &package_a, &lease_a, &result_a, &review_a).unwrap();
    let proposal_b =
        propose_acceptance(&binding(), &package_b, &lease_b, &result_b, &review_b).unwrap();
    engine.accept(11, &proposal_a).unwrap();
    engine.accept(12, &proposal_b).unwrap();
    (engine, proposal_a, proposal_b, journal)
}

fn batch_integration(
    proposal_a: &AcceptanceProposal,
    proposal_b: &AcceptanceProposal,
) -> RootIntegrationReceipt {
    let mut integration = integration_for(proposal_a, 13);
    integration.accepted_leases.insert(
        proposal_b.lease_id.clone(),
        AcceptedLeaseIntegration {
            node_id: proposal_b.node_id.clone(),
            proposal_id: proposal_b.proposal_id().unwrap(),
            requested_root_changes_digest: proposal_b.requested_root_changes_digest.clone(),
            requested_root_change_count: proposal_b.requested_root_change_count,
        },
    );
    integration
        .reconciled_request_digests
        .insert(proposal_b.requested_root_changes_digest.clone());
    integration
        .applied_changes
        .extend(proposal_b.expected_root_changes.clone());
    integration.integrated_bootstrap.completed_nodes.insert(
        proposal_b.node_id.clone(),
        proposal_b.proposal_id().unwrap(),
    );
    integration
}

fn batch_intent(
    proposal_a: &AcceptanceProposal,
    proposal_b: &AcceptanceProposal,
) -> RootIntegrationIntent {
    let mut expected = proposal_a.expected_root_changes.clone();
    expected.extend(proposal_b.expected_root_changes.clone());
    RootIntegrationIntent {
        schema_version: "RootIntegrationIntent-v1".to_owned(),
        base_binding: binding(),
        root_actor: "ultra-root".to_owned(),
        accepted_proposals: [proposal_a, proposal_b]
            .into_iter()
            .map(|proposal| (proposal.lease_id.clone(), proposal.proposal_id().unwrap()))
            .collect(),
        prior_digests: expected
            .keys()
            .map(|path| (path.clone(), Some(content_digest(LIVE_PRIOR_BYTES))))
            .collect(),
        expected_digests: expected,
    }
}

fn state_bytes(engine: &Orchestrator<CountingSink>) -> (EventLog, Vec<u8>, Binding) {
    (
        engine.event_log(),
        format!("{:?}", engine.projection).into_bytes(),
        engine.binding().clone(),
    )
}

#[test]
fn root_can_integrate_a_reviewed_disjoint_batch_in_one_rebind() {
    let (mut engine, proposal_a, proposal_b, _journal) =
        accepted_batch("plugin-manifest-draft.json");
    let integration = batch_integration(&proposal_a, &proposal_b);
    let intent = batch_intent(&proposal_a, &proposal_b);
    let workspace = live_workspace_for_intent(&intent);
    engine.begin_integration(13, intent).unwrap();
    engine
        .observe_integration(14, workspace.observer())
        .unwrap();
    let mut integration = integration;
    integration.integrated_bootstrap.observed_tick = 15;
    engine
        .complete(15, workspace.observer(), integration)
        .unwrap();
    assert!(engine.plan().unwrap().ready.is_empty());
}

#[test]
fn accepted_leases_cannot_overlap_exact_or_case_alias_root_changes() {
    for second_path in ["validator/src/lib.rs", "VALIDATOR/SRC/LIB.RS"] {
        let (mut engine, proposal_a, proposal_b, _journal) = accepted_batch(second_path);
        let intent = batch_intent(&proposal_a, &proposal_b);
        let before = state_bytes(&engine);
        assert!(engine.begin_integration(13, intent).is_err());
        assert_eq!(state_bytes(&engine), before);
    }
}
