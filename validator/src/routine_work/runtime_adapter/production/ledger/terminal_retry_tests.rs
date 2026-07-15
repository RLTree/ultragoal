use super::ledger_fixture_root::TestRoot;
use super::*;

fn retry_spec(token: &ReservationToken, label: &str) -> ReservationSpec {
    ReservationSpec {
        binding: token.binding.clone(),
        request_id: id(&format!("{label}-request")),
        grant_id: id(&format!("{label}-grant")),
        recovery_marker: id(&format!("{label}-recovery")),
        recovery_for: None,
        reuse_only: false,
        reuse_preauthorization: None,
    }
}

#[test]
fn failed_cancelled_and_incomplete_are_terminal_and_freshly_retryable() {
    for (label, terminal) in [
        ("failed-terminal", AttemptState::Failed),
        ("cancelled-terminal", AttemptState::Cancelled),
        ("incomplete-terminal", AttemptState::Incomplete),
    ] {
        let mut root = TestRoot::new(label);
        let ledger = FileAuthorityLedger::open_or_initialize(root.path()).unwrap();
        let token = reserve(&ledger, label);
        ledger.prepare_spawn(&token).unwrap();
        ledger.settle(&token, terminal, &BTreeMap::new()).unwrap();
        assert!(ledger.pending_recovery(&token.binding).unwrap().is_none());

        let replay = ledger.prepare_spawn(&token).unwrap_err();
        assert_eq!(replay.cause(), "routine-production-spawn-authority-invalid");
        drop(ledger);

        let ledger = FileAuthorityLedger::open_or_initialize(root.path()).unwrap();
        let retry = ledger
            .reserve(retry_spec(&token, &format!("{label}-retry")))
            .unwrap();
        let pending = ledger.pending_recovery(&retry.binding).unwrap().unwrap();
        assert_eq!(pending.marker, retry.recovery_marker);
        ledger
            .settle(&retry, AttemptState::Failed, &BTreeMap::new())
            .unwrap();
        assert!(ledger.pending_recovery(&retry.binding).unwrap().is_none());
        drop(ledger);
        root.teardown_after_assertions();
    }
}
