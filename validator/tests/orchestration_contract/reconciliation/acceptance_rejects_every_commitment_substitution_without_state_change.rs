#[test]
fn acceptance_rejects_every_commitment_substitution_without_state_change() {
    let (mut engine, proposal, _journal) = reviewed_engine();
    let before = state_bytes(&engine);
    let mut mutations = Vec::new();
    let rebind = |mut changed: AcceptanceProposal| {
        changed.result_commitment_id = changed.computed_result_commitment_id().unwrap();
        changed.validate().unwrap();
        changed
    };

    let mut changed = proposal.clone();
    *changed.artifact_digests.values_mut().next().unwrap() = digest('d');
    mutations.push(rebind(changed));
    let mut changed = proposal.clone();
    changed.artifact_digests.clear();
    mutations.push(rebind(changed));
    let mut changed = proposal.clone();
    changed.artifact_digests.insert(
        "validator/src/orchestration/extra.rs".to_owned(),
        digest('d'),
    );
    mutations.push(rebind(changed));
    let mut changed = proposal.clone();
    changed.requested_root_change_count += 1;
    mutations.push(changed);
    let mut changed = proposal.clone();
    changed.requested_root_changes_digest = digest('d');
    mutations.push(changed);
    let mut changed = proposal.clone();
    changed.node_id = "node-b".to_owned();
    mutations.push(rebind(changed));
    let mut changed = proposal.clone();
    changed.lease_id = "lease-002".to_owned();
    mutations.push(rebind(changed));
    let mut changed = proposal.clone();
    changed.binding = Binding::new(&digest('d'), &digest('e')).unwrap();
    mutations.push(rebind(changed));
    let mut changed = proposal.clone();
    changed.result_id = digest('d');
    mutations.push(rebind(changed));
    let mut changed = proposal.clone();
    changed.review_id = digest('d');
    mutations.push(changed);
    let mut changed = proposal.clone();
    changed.result_commitment_id = digest('d');
    mutations.push(changed);

    for changed in mutations {
        assert!(engine.accept(6, &changed).is_err());
        assert_eq!(state_bytes(&engine), before);
    }

    engine.accept(6, &proposal).unwrap();
    let workspace = prepare_full_integration(&mut engine, &proposal, 7);
    engine
        .complete(9, workspace.observer(), integration_for(&proposal, 9))
        .unwrap();
}

#[test]
fn replay_rejects_reidentified_submit_review_and_accept_commitment_tampering() {
    let (mut engine, _) = engine();
    let lease = lease();
    let package = package("node-a", &[], "node_a");
    let result = result_for(&lease, &package);
    let review = review_for(&result);
    let proposal = propose_acceptance(&binding(), &package, &lease, &result, &review).unwrap();
    engine.grant_lease(1, lease).unwrap();
    engine.start(2, "lease-001").unwrap();
    engine.submit_structural(3, "lease-001", &result).unwrap();

    let tampered = tamper_last_event(engine.event_log(), |event| {
        let commitment = event
            .get_mut("commitment")
            .expect("submit event must persist the result commitment");
        commitment["artifact_digests"]["validator/src/orchestration/node_a.rs"] =
            serde_json::json!(digest('d'));
    });
    let (sink, _) = CountingSink::new();
    assert!(
        Orchestrator::restart(graph_one(), policy(), binding(), root(), tampered, sink).is_err()
    );

    engine
        .assign_review(4, "lease-001", worker("reviewer-a"))
        .unwrap();
    engine.record_review(5, "lease-001", review).unwrap();
    let tampered = tamper_last_event(engine.event_log(), |event| {
        event["result_commitment_id"] = serde_json::json!(digest('d'));
    });
    let (sink, _) = CountingSink::new();
    assert!(
        Orchestrator::restart(graph_one(), policy(), binding(), root(), tampered, sink).is_err()
    );

    engine.accept(6, &proposal).unwrap();
    let tampered = tamper_last_event(engine.event_log(), |event| {
        let proposal = event.get_mut("proposal").unwrap();
        proposal["expected_root_changes"]["validator/src/lib.rs"] = serde_json::json!(digest('d'));
    });
    let (sink, _) = CountingSink::new();
    assert!(
        Orchestrator::restart(graph_one(), policy(), binding(), root(), tampered, sink).is_err()
    );
}
