use crate::durable_support::*;
use crate::orchestration::*;
use crate::support::*;
use std::collections::BTreeSet;

fn started_engine() -> Orchestrator<CountingSink> {
    let (mut engine, _) = engine();
    engine.grant_lease(1, lease()).unwrap();
    engine.start(2, "lease-001").unwrap();
    engine
}

fn retry_subject() -> (LeaseSpec, WorkPackage, WorkerResultV1) {
    let lease = lease();
    let package = package("node-a", &[], "node_a");
    let result = result_for(&lease, &package);
    (lease, package, result)
}

#[test]
fn interrupted_root_replays_and_recovers_deterministically() {
    let mut original = started_engine();
    original.interrupt_root(3).unwrap();
    assert_eq!(
        original.heartbeat(4, "lease-001").unwrap_err(),
        OrchestrationError::InvalidTransition
    );
    let log = original.event_log();
    let (sink, _) = CountingSink::new();
    let mut restarted =
        Orchestrator::restart(graph_one(), policy(), binding(), root(), log.clone(), sink).unwrap();
    assert_eq!(restarted.event_log(), log);
    restarted.recover_root(5).unwrap();
    restarted.heartbeat(6, "lease-001").unwrap();
}

#[test]
fn tampered_event_identity_is_rejected_on_restart() {
    let mut log = started_engine().event_log();
    let mut unknown = serde_json::to_value(&log).unwrap();
    unknown[0]["event"]["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<EventLog>(unknown).is_err());
    log.0[0].logical_tick = 9;
    let (sink, _) = CountingSink::new();
    let error = Orchestrator::restart(graph_one(), policy(), binding(), root(), log, sink)
        .err()
        .unwrap();
    assert_eq!(error, OrchestrationError::InvalidEvent);
}

#[test]
fn deleted_event_breaks_chain() {
    let engine = started_engine();
    let mut log = engine.event_log();
    log.0.remove(0);
    let (sink, _) = CountingSink::new();
    assert_eq!(
        Orchestrator::restart(graph_one(), policy(), binding(), root(), log, sink)
            .err()
            .unwrap(),
        OrchestrationError::ReplayMismatch
    );
}

#[test]
fn nonmonotonic_logical_time_is_rejected_without_appending() {
    let mut engine = started_engine();
    let count = engine.events().len();
    assert_eq!(
        engine.heartbeat(1, "lease-001").unwrap_err(),
        OrchestrationError::ReplayMismatch
    );
    assert_eq!(engine.events().len(), count);
}

#[test]
fn lease_must_be_a_subset_of_its_work_package() {
    let (mut engine, _) = engine();
    let mut overbroad = lease();
    overbroad.owned_scope.paths = [path("validator/src/orchestration")].into();
    assert_eq!(
        engine.grant_lease(1, overbroad).unwrap_err(),
        OrchestrationError::InvalidLease
    );
    assert_eq!(engine.events().len(), 1);
}

#[test]
fn lease_cannot_bypass_required_tool_or_prerequisite_evidence() {
    let (mut engine, _) = engine();
    let mut missing = lease();
    missing.prerequisite_evidence.required_tools.clear();
    assert_eq!(
        engine.grant_lease(1, missing).unwrap_err(),
        OrchestrationError::InvalidLease
    );
    let mut unknown = lease();
    unknown
        .prerequisite_evidence
        .prerequisites
        .insert("invented".to_owned(), digest('7'));
    assert_eq!(
        engine.grant_lease(1, unknown).unwrap_err(),
        OrchestrationError::InvalidLease
    );
    let mut stale = lease();
    stale
        .prerequisite_evidence
        .required_tools
        .insert("hct-context".to_owned(), digest('7'));
    assert_eq!(
        engine.grant_lease(1, stale).unwrap_err(),
        OrchestrationError::InvalidLease
    );
}

#[test]
fn recovery_classifies_stale_expired_orphan_and_resumable() {
    let engine = started_engine();
    let live = BTreeSet::from(["worker-a".to_owned()]);
    assert_eq!(
        engine
            .recovery_report(&Binding::new(&digest('e'), &digest('f')).unwrap(), 3, &live)
            .stale_binding_leases,
        ["lease-001"]
    );
    assert_eq!(
        engine.recovery_report(&binding(), 21, &live).expired_leases,
        ["lease-001"]
    );
    assert_eq!(
        engine
            .recovery_report(&binding(), 3, &BTreeSet::new())
            .orphaned_leases,
        ["lease-001"]
    );
    assert_eq!(
        engine
            .recovery_report(&binding(), 3, &live)
            .resumable_leases,
        ["lease-001"]
    );
}

#[test]
fn bounded_rework_retry_exhausts_exact_allowance() {
    let mut engine = started_engine();
    let (_, _, result) = retry_subject();
    for retry in 0..2 {
        engine
            .submit_structural(3 + retry * 5, "lease-001", &result)
            .unwrap();
        engine
            .assign_review(4 + retry * 5, "lease-001", worker("reviewer-a"))
            .unwrap();
        engine
            .record_review(
                5 + retry * 5,
                "lease-001",
                review_for_result(&result, ReviewDecision::Rework),
            )
            .unwrap();
        engine
            .schedule_retry(6 + retry * 5, "lease-001", 30 + retry * 10)
            .unwrap();
        engine.start(7 + retry * 5, "lease-001").unwrap();
    }
    engine.submit_structural(20, "lease-001", &result).unwrap();
    engine
        .assign_review(21, "lease-001", worker("reviewer-a"))
        .unwrap();
    engine
        .record_review(
            22,
            "lease-001",
            review_for_result(&result, ReviewDecision::Rework),
        )
        .unwrap();
    assert_eq!(
        engine.schedule_retry(23, "lease-001", 60).unwrap_err(),
        OrchestrationError::RetryExhausted
    );
}

#[test]
fn cancellation_releases_active_node_and_prevents_resume() {
    let mut engine = started_engine();
    engine.cancel(3, "lease-001").unwrap();
    assert_eq!(
        engine.start(4, "lease-001").unwrap_err(),
        OrchestrationError::InvalidTransition
    );
    assert_eq!(engine.plan().unwrap().ready, ["node-a"]);
}

#[test]
fn forged_acceptance_proposal_is_rejected() {
    let mut engine = started_engine();
    let (_, _, result) = retry_subject();
    let review = review_for_result(&result, ReviewDecision::Pass);
    engine.submit_structural(3, "lease-001", &result).unwrap();
    engine
        .assign_review(4, "lease-001", worker("reviewer-a"))
        .unwrap();
    engine
        .record_review(5, "lease-001", review.clone())
        .unwrap();
    let forged = AcceptanceProposal {
        schema_version: "AcceptanceProposal-v1".to_owned(),
        binding: binding(),
        node_id: "node-a".to_owned(),
        lease_id: "lease-001".to_owned(),
        result_id: digest('e'),
        result_commitment_id: digest('f'),
        review_id: review.review_id().unwrap(),
        artifact_digests: Default::default(),
        expected_root_changes: Default::default(),
        requested_root_change_count: 0,
        requested_root_changes_digest:
            "sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a".to_owned(),
        root_decision_required: true,
    };
    assert_eq!(
        engine.accept(6, &forged).unwrap_err(),
        OrchestrationError::InvalidReview
    );
}

#[test]
fn malformed_review_payload_cannot_enter_replay_log() {
    let mut engine = started_engine();
    let (_, _, result) = retry_subject();
    engine.submit_structural(3, "lease-001", &result).unwrap();
    engine
        .assign_review(4, "lease-001", worker("reviewer-a"))
        .unwrap();
    let mut forged = review_for_result(&result, ReviewDecision::Pass);
    forged.worker = "different-worker".to_owned();
    let count = engine.events().len();
    assert_eq!(
        engine.record_review(5, "lease-001", forged).unwrap_err(),
        OrchestrationError::ReviewerNotIndependent
    );
    assert_eq!(engine.events().len(), count);
}

#[test]
fn recomputed_attacker_root_change_commitment_is_not_the_reviewed_result() {
    let mut engine = started_engine();
    let (lease, package, result) = retry_subject();
    let review = review_for_result(&result, ReviewDecision::Pass);
    let mut proposal = propose_acceptance(&binding(), &package, &lease, &result, &review).unwrap();
    engine.submit_structural(3, "lease-001", &result).unwrap();
    engine
        .assign_review(4, "lease-001", worker("reviewer-a"))
        .unwrap();
    engine.record_review(5, "lease-001", review).unwrap();
    *proposal.expected_root_changes.values_mut().next().unwrap() = digest('7');
    proposal.requested_root_change_count = proposal.expected_root_changes.len();
    proposal.requested_root_changes_digest = proposal.computed_root_changes_digest().unwrap();
    proposal.result_commitment_id = proposal.computed_result_commitment_id().unwrap();
    proposal.validate().unwrap();
    let before_log = engine.event_log();
    let before_plan = engine.plan().unwrap();
    assert_eq!(
        engine.accept(6, &proposal).unwrap_err(),
        OrchestrationError::InvalidReview
    );
    assert_eq!(engine.event_log(), before_log);
    assert_eq!(engine.plan().unwrap(), before_plan);
}
