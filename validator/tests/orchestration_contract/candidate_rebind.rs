use crate::durable_support::*;
use crate::orchestration::*;
use crate::support::*;

#[test]
fn integrated_candidate_replays_only_at_its_final_binding() {
    let (mut engine, _, _journal) = durable_engine("rebind-final-binding");
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
    let workspace = prepare_full_integration(&mut engine, &proposal, 7);
    engine
        .complete(9, workspace.observer(), integration_for(&proposal, 9))
        .unwrap();

    let final_binding = engine.binding().clone();
    assert_ne!(final_binding, binding());
    let log = engine.event_log();
    let (sink, _) = CountingSink::new();
    let restarted = Orchestrator::restart(
        graph_one(),
        policy(),
        final_binding.clone(),
        root(),
        log.clone(),
        sink,
    )
    .unwrap();
    assert_eq!(restarted.binding(), &final_binding);
    let (sink, _) = CountingSink::new();
    assert_eq!(
        Orchestrator::restart(graph_one(), policy(), binding(), root(), log, sink)
            .err()
            .unwrap(),
        OrchestrationError::ReplayMismatch
    );
}

#[test]
fn candidate_rebind_invalidates_every_other_active_lease() {
    let graph = WorkGraph::derive(vec![
        package("node-a", &[], "node_a"),
        package("node-b", &[], "node_b"),
    ])
    .unwrap();
    let journal = JournalRoot::new("rebind-invalidates");
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
    let package_a = package("node-a", &[], "node_a");
    let result = result_for(&lease_a, &package_a);
    engine.grant_lease(1, lease_a.clone()).unwrap();
    let mut lease_b = lease_with_scope("lease-002", "node-b", "worker-b", scope("node_b"));
    lease_b.issued_tick = 2;
    engine.grant_lease(2, lease_b).unwrap();
    engine.start(3, "lease-001").unwrap();
    engine.start(4, "lease-002").unwrap();
    engine.submit_structural(5, "lease-001", &result).unwrap();
    engine
        .assign_review(6, "lease-001", worker("reviewer-a"))
        .unwrap();
    let review = review_for_result(&result, ReviewDecision::Pass);
    engine
        .record_review(7, "lease-001", review.clone())
        .unwrap();
    let proposal = propose_acceptance(&binding(), &package_a, &lease_a, &result, &review).unwrap();
    engine.accept(8, &proposal).unwrap();
    let workspace = prepare_full_integration(&mut engine, &proposal, 9);
    let mut integration = integration_for(&proposal, 11);
    integration
        .invalidated_lease_ids
        .insert("lease-002".to_owned());
    engine
        .complete(11, workspace.observer(), integration)
        .unwrap();

    assert_eq!(
        engine.heartbeat(12, "lease-002").unwrap_err(),
        OrchestrationError::InvalidTransition
    );
    assert_eq!(engine.plan().unwrap().ready, ["node-b"]);
}

fn root_change(path: &str, bytes: &[u8]) -> RootChangeRequest {
    RootChangeRequest::new(
        path,
        &content_digest(bytes),
        Some("exact expected root output"),
    )
    .unwrap()
}

fn reviewed_single(
    requests: Vec<RootChangeRequest>,
) -> (Orchestrator<CountingSink>, AcceptanceProposal, JournalRoot) {
    let lease = lease();
    let package = package("node-a", &[], "node_a");
    let mut result = result_for(&lease, &package);
    result.requested_root_changes = requests;
    let review = review_for_result(&result, ReviewDecision::Pass);
    let proposal = propose_acceptance(&binding(), &package, &lease, &result, &review).unwrap();
    let (mut engine, _, journal) = durable_engine("reviewed-single");
    engine.grant_lease(1, lease).unwrap();
    engine.start(2, "lease-001").unwrap();
    engine.submit_structural(3, "lease-001", &result).unwrap();
    engine
        .assign_review(4, "lease-001", worker("reviewer-a"))
        .unwrap();
    engine.record_review(5, "lease-001", review).unwrap();
    (engine, proposal, journal)
}

fn accepted_single(
    requests: Vec<RootChangeRequest>,
) -> (Orchestrator<CountingSink>, AcceptanceProposal, JournalRoot) {
    let (mut engine, proposal, journal) = reviewed_single(requests);
    engine.accept(6, &proposal).unwrap();
    (engine, proposal, journal)
}

fn state_bytes(engine: &Orchestrator<CountingSink>) -> (EventLog, Vec<u8>, Binding) {
    (
        engine.event_log(),
        format!("{:?}", engine.projection).into_bytes(),
        engine.binding().clone(),
    )
}

#[test]
fn zero_and_multiple_exact_root_change_sets_complete() {
    for requests in [
        Vec::new(),
        vec![
            root_change("validator/src/lib.rs", LIVE_LIB_BYTES),
            root_change("plugin-manifest-draft.json", LIVE_MANIFEST_BYTES),
        ],
    ] {
        let expected: std::collections::BTreeMap<_, _> = requests
            .iter()
            .map(|item| (item.path.clone(), item.expected_sha256.clone()))
            .collect();
        let (mut engine, proposal, _journal) = accepted_single(requests);
        let workspace = prepare_full_integration(&mut engine, &proposal, 7);
        let integration = integration_for(&proposal, 9);
        assert_eq!(integration.applied_changes, expected);
        if expected.is_empty() {
            let mut extra = integration.clone();
            extra
                .applied_changes
                .insert("extra.json".to_owned(), digest('7'));
            assert!(engine.complete(9, workspace.observer(), extra).is_err());
        }
        engine
            .complete(9, workspace.observer(), integration)
            .unwrap();
    }
}

#[test]
fn rebound_rejects_every_applied_set_substitution_without_state_change() {
    let (mut engine, proposal, _journal) =
        accepted_single(vec![root_change("validator/src/lib.rs", LIVE_LIB_BYTES)]);
    let workspace = prepare_full_integration(&mut engine, &proposal, 7);
    let valid = integration_for(&proposal, 9);
    let mut mutations = Vec::new();
    let mut changed = valid.clone();
    changed.applied_changes.clear();
    mutations.push(changed);
    let mut changed = valid.clone();
    *changed.applied_changes.values_mut().next().unwrap() = digest('7');
    mutations.push(changed);
    let mut changed = valid.clone();
    changed.applied_changes.clear();
    changed
        .applied_changes
        .insert("other.json".to_owned(), digest('6'));
    mutations.push(changed);
    let mut changed = valid.clone();
    changed
        .applied_changes
        .insert("extra.json".to_owned(), digest('7'));
    mutations.push(changed);
    let mut changed = valid.clone();
    changed
        .accepted_leases
        .values_mut()
        .next()
        .unwrap()
        .requested_root_change_count += 1;
    mutations.push(changed);
    let mut changed = valid.clone();
    changed
        .accepted_leases
        .values_mut()
        .next()
        .unwrap()
        .requested_root_changes_digest = digest('7');
    mutations.push(changed);

    let before = state_bytes(&engine);
    for changed in mutations {
        assert!(engine.complete(9, workspace.observer(), changed).is_err());
        assert_eq!(state_bytes(&engine), before);
    }
    engine.complete(9, workspace.observer(), valid).unwrap();
}
