use super::ledger_fixture_root::TestRoot;
use super::*;

#[test]
fn unstaged_recovery_requires_reexecution_before_artifact_authentication() {
    let mut root = TestRoot::new("unstaged-recovery-authentication");
    let ledger = FileAuthorityLedger::open_or_initialize(root.path()).unwrap();
    let first = reserve(&ledger, "unstaged-recovery-authentication");
    let artifacts = BTreeMap::from([(
        id("unstaged-recovery-artifact"),
        id("unstaged-recovery-witness"),
    )]);
    ledger.prepare_spawn(&first).unwrap();
    let pending = ledger.pending_recovery(&first.binding).unwrap().unwrap();
    drop(ledger);

    let ledger = FileAuthorityLedger::open_existing(root.path()).unwrap();
    let recovered = ledger
        .reserve(ReservationSpec {
            binding: first.binding.clone(),
            request_id: first.request_id.clone(),
            grant_id: id("unstaged-recovery-grant-two"),
            recovery_marker: id("unstaged-recovery-marker-two"),
            recovery_for: Some(pending.marker),
            reuse_only: false,
            reuse_preauthorization: None,
        })
        .unwrap();
    let (digest, witness) = artifacts.first_key_value().unwrap();
    assert!(!ledger.authenticates(&recovered, digest, witness).unwrap());
    ledger.prepare_spawn(&recovered).unwrap();
    ledger.stage_success(&recovered, &artifacts).unwrap();
    assert!(ledger.authenticates(&recovered, digest, witness).unwrap());
    assert!(
        !ledger
            .authenticates(&recovered, digest, &id("foreign-witness"))
            .unwrap()
    );
    ledger
        .settle(&recovered, AttemptState::Complete, &artifacts)
        .unwrap();
    assert!(ledger.pending_recovery(&first.binding).unwrap().is_none());
    drop(ledger);
    root.teardown_after_assertions();
}
