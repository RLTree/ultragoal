use super::*;

#[test]
pub(crate) fn authenticated_device_drift_is_diagnosed_without_rewriting_history() {
    let fixture = Fixture::new("device-drift-diagnosis");
    let ledger =
        FileRepositoryFitLedger::open_or_initialize(&fixture.store.root, fixture.store.store_id())
            .unwrap();
    assert!(
        FileRepositoryFitLedger::assess_device_drift(&fixture.store.root, fixture.store.store_id())
            .unwrap()
            .is_none()
    );

    let token = match ledger.reserve(reservation('1')).unwrap() {
        ReservationDecision::Acquired(token) => token,
        ReservationDecision::Existing(_) => unreachable!(),
    };
    ledger
        .terminal(
            token,
            RepositoryFitLedgerState::Rejected,
            &fixed_digest('9'),
            Some(AdapterErrorId::ApplyPermitInvalid),
            11,
        )
        .unwrap();
    let current_device = fs::metadata(&fixture.store.root).unwrap().dev();
    simulate_device_drift_for_test(&fixture.store.root, current_device + 1).unwrap();

    let first =
        FileRepositoryFitLedger::assess_device_drift(&fixture.store.root, fixture.store.store_id())
            .unwrap()
            .unwrap();
    let second =
        FileRepositoryFitLedger::assess_device_drift(&fixture.store.root, fixture.store.store_id())
            .unwrap()
            .unwrap();
    assert_eq!(first, second);
    assert_eq!(first.reservation_count, 1);
    assert_eq!(first.nonterminal_reservation_count, 0);
    assert_eq!(first.stored_root.device, current_device + 1);
    assert_eq!(first.current_root.device, current_device);
    assert_eq!(first.stored_lock.device, current_device + 1);
    assert_eq!(first.current_lock.device, current_device);
    assert_eq!(first.stored_root.inode, first.current_root.inode);
    assert_eq!(first.stored_lock.inode, first.current_lock.inode);

    let state = fixture.store.root.join("authority-ledger.json");
    let bytes = String::from_utf8(fs::read(&state).unwrap()).unwrap();
    fs::write(
        &state,
        bytes.replacen("\"generation\":2", "\"generation\":3", 1),
    )
    .unwrap();
    assert!(matches!(
        FileRepositoryFitLedger::assess_device_drift(
            &fixture.store.root,
            fixture.store.store_id()
        ),
        Err(ref error) if error.id() == LedgerErrorId::Tampered
    ));
}
