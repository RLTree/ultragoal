use super::*;

#[test]
fn complete_effect_can_reexecute_after_a_cache_miss() {
    let mut root = TestRoot::new("complete-reexecute");
    let ledger = FileAuthorityLedger::open_or_initialize(root.path()).unwrap();
    let first = reserve(&ledger, "complete-reexecute");
    let artifacts = BTreeMap::from([(id("first-artifact"), id("first-witness"))]);
    ledger.prepare_spawn(&first).unwrap();
    ledger.stage_success(&first, &artifacts).unwrap();
    ledger
        .settle(&first, AttemptState::Complete, &artifacts)
        .unwrap();

    let second = ledger
        .reserve(ReservationSpec {
            binding: first.binding.clone(),
            request_id: id("complete-reexecute-request-two"),
            grant_id: id("complete-reexecute-grant-two"),
            recovery_marker: id("complete-reexecute-recovery-two"),
            recovery_for: None,
            reuse_only: false,
            reuse_preauthorization: None,
        })
        .unwrap();
    ledger.prepare_spawn(&second).unwrap();
    let replacement = BTreeMap::from([(id("second-artifact"), id("second-witness"))]);
    ledger.stage_success(&second, &replacement).unwrap();
    ledger
        .settle(&second, AttemptState::Complete, &replacement)
        .unwrap();
    assert!(ledger.pending_recovery(&second.binding).unwrap().is_none());
    drop(ledger);
    root.teardown_after_assertions();
}
