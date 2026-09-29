use super::durable_journal_fixture::*;
use super::orchestration_fixture::*;
use crate::orchestration::*;
use std::collections::BTreeSet;

fn request(class: EffectClass, target: &str) -> EffectRequest {
    EffectRequest {
        lease_id: "lease-001".to_owned(),
        binding: binding(),
        effect: effect(class, target),
        operation_id: "operation-001".to_owned(),
        payload_digest: digest('e'),
    }
}

#[test]
fn malformed_sink_receipt_leaves_durable_ambiguous_intent() {
    let (mut sink, calls) = CountingSink::new();
    sink.mismatch = true;
    let journal = JournalRoot::new("effect-malformed-receipt");
    let mut engine = Orchestrator::new_durable(
        graph_one(),
        policy(),
        binding(),
        root(),
        bootstrap(),
        journal.path(),
        sink,
    )
    .unwrap();
    engine.grant_lease(1, lease()).unwrap();
    engine.start(2, "lease-001").unwrap();
    let before = engine.events().len();
    assert_eq!(
        engine
            .apply_effect(
                3,
                request(EffectClass::WorkspaceWrite, "leased-source/node_a")
            )
            .unwrap_err(),
        OrchestrationError::EffectAmbiguous
    );
    assert_eq!(calls.get(), 1);
    assert_eq!(engine.events().len(), before + 1);
    assert!(matches!(
        engine.events().last().unwrap().event,
        EventKind::EffectIntent { .. }
    ));
    assert_eq!(
        engine
            .recovery_report(&binding(), 3, &BTreeSet::new())
            .ambiguous_operations,
        ["operation-001"]
    );
}

#[test]
fn failed_effect_cannot_retry_and_root_reconciliation_survives_restart() {
    let (mut sink, calls) = CountingSink::new();
    sink.fail = true;
    let journal = JournalRoot::new("effect-failure-restart");
    let mut engine = Orchestrator::new_durable(
        graph_one(),
        policy(),
        binding(),
        root(),
        bootstrap(),
        journal.path(),
        sink,
    )
    .unwrap();
    engine.grant_lease(1, lease()).unwrap();
    engine.start(2, "lease-001").unwrap();
    let request = request(EffectClass::WorkspaceWrite, "leased-source/node_a");
    assert_eq!(
        engine.apply_effect(3, request.clone()).unwrap_err(),
        OrchestrationError::EffectAmbiguous
    );
    assert_eq!(
        engine.apply_effect(4, request).unwrap_err(),
        OrchestrationError::EffectDenied
    );
    assert_eq!(calls.get(), 1);
    let expected_head = engine.journal_head().unwrap().clone();
    drop(engine);

    let (sink, restarted_calls) = CountingSink::new();
    let mut restarted = Orchestrator::restart_durable(
        graph_one(),
        policy(),
        expected_head,
        root(),
        journal.path(),
        sink,
    )
    .unwrap();
    assert_eq!(
        restarted
            .recovery_report(&binding(), 4, &BTreeSet::new())
            .ambiguous_operations,
        ["operation-001"]
    );
    restarted
        .reconcile_effect(
            4,
            "lease-001",
            EffectResolution {
                operation_id: "operation-001".to_owned(),
                evidence_digest: digest('7'),
                outcome: EffectOutcome::NotApplied,
            },
        )
        .unwrap();
    assert!(
        restarted
            .recovery_report(&binding(), 4, &BTreeSet::new())
            .ambiguous_operations
            .is_empty()
    );
    assert_eq!(restarted_calls.get(), 0);
}
