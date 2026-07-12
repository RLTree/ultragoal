use crate::durable_support::*;
use crate::orchestration::*;
use crate::support::*;
use serde_json::Value;
use std::collections::BTreeSet;

fn review_for(result: &WorkerResultV1) -> ReviewRecord {
    ReviewRecord {
        reviewer: "reviewer-a".to_owned(),
        worker: result.worker.clone(),
        binding: binding(),
        result_id: result.result_id().unwrap(),
        result_commitment_id: commitment_id(result),
        decision: ReviewDecision::Pass,
        reproduced_commands: BTreeSet::from(["cargo-test-focused".to_owned()]),
        finding_codes: BTreeSet::new(),
    }
}

pub(crate) fn reviewed_engine() -> (Orchestrator<CountingSink>, AcceptanceProposal, JournalRoot) {
    let (mut engine, _, journal) = durable_engine("reconciled-engine");
    let lease = lease();
    let package = package("node-a", &[], "node_a");
    let result = result_for(&lease, &package);
    let review = review_for(&result);
    let proposal = propose_acceptance(&binding(), &package, &lease, &result, &review).unwrap();
    engine.grant_lease(1, lease).unwrap();
    engine.start(2, "lease-001").unwrap();
    engine.submit_structural(3, "lease-001", &result).unwrap();
    engine
        .assign_review(4, "lease-001", worker("reviewer-a"))
        .unwrap();
    engine.record_review(5, "lease-001", review).unwrap();
    (engine, proposal, journal)
}

fn state_bytes(engine: &Orchestrator<CountingSink>) -> (Vec<u8>, Vec<u8>) {
    (
        serde_json::to_vec(&engine.event_log()).unwrap(),
        format!("{:?}", engine.projection).into_bytes(),
    )
}

fn tamper_last_event(mut log: EventLog, mutate: impl FnOnce(&mut Value)) -> EventLog {
    let mut encoded = serde_json::to_value(&log).unwrap();
    let event = encoded
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap()
        .get_mut("event")
        .unwrap();
    mutate(event);
    log = serde_json::from_value(encoded).unwrap();
    let index = log.0.len() - 1;
    let prior = log.0[index].clone();
    log.0[index] = OrchestrationEvent::create(
        prior.sequence,
        prior.prior_event_id,
        prior.binding,
        prior.actor,
        prior.logical_tick,
        prior.event,
    )
    .unwrap();
    log
}

#[test]
fn independent_pass_yields_deterministic_root_only_proposal() {
    let lease = lease();
    let package = package("node-a", &[], "node_a");
    let result = result_for(&lease, &package);
    let review = review_for(&result);
    let first = propose_acceptance(&binding(), &package, &lease, &result, &review).unwrap();
    let second = propose_acceptance(&binding(), &package, &lease, &result, &review).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.proposal_id().unwrap(), second.proposal_id().unwrap());
    assert!(first.root_decision_required);
}

#[test]
fn worker_cannot_review_own_result() {
    let lease = lease();
    let package = package("node-a", &[], "node_a");
    let result = result_for(&lease, &package);
    let mut review = review_for(&result);
    review.reviewer = result.worker.clone();
    assert_eq!(
        propose_acceptance(&binding(), &package, &lease, &result, &review).unwrap_err(),
        OrchestrationError::ReviewerNotIndependent
    );
}

#[test]
fn stale_review_binding_is_rejected() {
    let lease = lease();
    let package = package("node-a", &[], "node_a");
    let result = result_for(&lease, &package);
    let mut review = review_for(&result);
    review.binding = Binding::new(&digest('d'), &digest('e')).unwrap();
    assert_eq!(
        propose_acceptance(&binding(), &package, &lease, &result, &review).unwrap_err(),
        OrchestrationError::StaleBinding
    );
}

#[test]
fn rework_or_unresolved_dependency_cannot_be_accepted() {
    let lease = lease();
    let package = package("node-a", &[], "node_a");
    let mut result = result_for(&lease, &package);
    let mut review = review_for(&result);
    review.decision = ReviewDecision::Rework;
    assert_eq!(
        propose_acceptance(&binding(), &package, &lease, &result, &review).unwrap_err(),
        OrchestrationError::InvalidReview
    );

    result.unresolved_dependencies = vec!["root-wiring".to_owned()];
    let review = review_for(&result);
    assert_eq!(
        propose_acceptance(&binding(), &package, &lease, &result, &review).unwrap_err(),
        OrchestrationError::InvalidReview
    );
}

#[test]
fn review_must_reproduce_material_evidence() {
    let lease = lease();
    let package = package("node-a", &[], "node_a");
    let result = result_for(&lease, &package);
    let mut review = review_for(&result);
    review.reproduced_commands.clear();
    assert_eq!(
        review.validate().unwrap_err(),
        OrchestrationError::InvalidReview
    );

    review.reproduced_commands =
        BTreeSet::from(["cargo test --test orchestration_contract --locked".to_owned()]);
    review.validate().unwrap();
    review.reproduced_commands = BTreeSet::from(["cargo test\nforged".to_owned()]);
    assert_eq!(
        review.validate().unwrap_err(),
        OrchestrationError::InvalidReview
    );
}

#[test]
fn duplicate_artifact_authority_is_rejected() {
    let lease = lease();
    let package = package("node-a", &[], "node_a");
    let mut result = result_for(&lease, &package);
    let review = review_for(&result);
    result.artifacts.push(result.artifacts[0].clone());
    assert_eq!(
        propose_acceptance(&binding(), &package, &lease, &result, &review).unwrap_err(),
        OrchestrationError::DuplicateOutput
    );
}

#[test]
fn malformed_acceptance_schema_is_rejected() {
    let lease = lease();
    let package = package("node-a", &[], "node_a");
    let result = result_for(&lease, &package);
    let review = review_for(&result);
    let mut proposal = propose_acceptance(&binding(), &package, &lease, &result, &review).unwrap();
    proposal.schema_version = "future-unknown".to_owned();
    assert_eq!(
        proposal.proposal_id().unwrap_err(),
        OrchestrationError::InvalidReview
    );
}

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
