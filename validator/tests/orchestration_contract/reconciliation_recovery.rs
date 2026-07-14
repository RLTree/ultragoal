use crate::durable_journal_fixture::*;
use crate::orchestration::*;
use crate::orchestration_fixture::*;
use crate::reconciliation::reviewed_engine;

#[test]
fn reviewed_commitment_survives_interrupted_restart_byte_identically() {
    let (mut original, proposal, journal) = reviewed_engine();
    original.interrupt_root(6).unwrap();
    let log = original.event_log();
    let bytes = serde_json::to_vec(&log).unwrap();
    let expected_head = original.journal_head().unwrap().clone();
    drop(original);
    let (sink, _) = CountingSink::new();
    let mut restarted = Orchestrator::restart_durable(
        graph_one(),
        policy(),
        expected_head,
        root(),
        journal.path(),
        sink,
    )
    .unwrap();
    assert_eq!(serde_json::to_vec(&restarted.event_log()).unwrap(), bytes);
    restarted.recover_root(7).unwrap();
    restarted.accept(8, &proposal).unwrap();
    let workspace = prepare_full_integration(&mut restarted, &proposal, 9);
    restarted
        .complete(11, workspace.observer(), integration_for(&proposal, 11))
        .unwrap();
}
