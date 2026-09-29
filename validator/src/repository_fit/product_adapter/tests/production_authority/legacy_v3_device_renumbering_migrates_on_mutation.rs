use super::*;

#[test]
pub(crate) fn authenticated_legacy_device_drift_opens_without_write_and_migrates_on_mutation() {
    let fixture = Fixture::new("device-drift-device-neutral-migration");
    let ledger =
        FileRepositoryFitLedger::open_or_initialize(&fixture.store.root, fixture.store.store_id())
            .unwrap();

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
    write_legacy_v3_device_number_for_test(&fixture.store.root, current_device + 1).unwrap();

    let state = fixture.store.root.join("authority-ledger.json");
    let legacy_bytes = fs::read(&state).unwrap();
    let reopened =
        FileRepositoryFitLedger::open_existing(&fixture.store.root, fixture.store.store_id())
            .unwrap();
    assert_eq!(fs::read(&state).unwrap(), legacy_bytes);
    assert!(
        reopened
            .lookup_by_nonce(&fixed_digest('5'))
            .unwrap()
            .is_some()
    );

    let mutating =
        FileRepositoryFitLedger::open_or_initialize(&fixture.store.root, fixture.store.store_id())
            .unwrap();
    assert_eq!(fs::read(&state).unwrap(), legacy_bytes);
    let second = match mutating.reserve(reservation('2')).unwrap() {
        ReservationDecision::Acquired(token) => token,
        ReservationDecision::Existing(_) => unreachable!(),
    };
    mutating
        .terminal(
            second,
            RepositoryFitLedgerState::Rejected,
            &fixed_digest('8'),
            Some(AdapterErrorId::ApplyPermitInvalid),
            11,
        )
        .unwrap();
    let migrated = String::from_utf8(fs::read(&state).unwrap()).unwrap();
    assert!(migrated.contains("repository-fit-authority-ledger-envelope.v4"));
    assert!(migrated.contains("repository-fit-authority-ledger.v4"));
    let migrated_value: serde_json::Value = serde_json::from_str(&migrated).unwrap();
    assert!(
        migrated_value["payload"]["root_identity"]
            .get("device")
            .is_none()
    );
    assert!(
        migrated_value["payload"]["lock_identity"]
            .get("device")
            .is_none()
    );
    assert!(migrated.contains("\"predecessor_envelope_sha256\":\"sha256:"));

    // A later boot-like device renumbering remains a compatibility read, not a
    // quarantine trigger or hidden write.
    write_legacy_v3_device_number_for_test(&fixture.store.root, current_device + 2).unwrap();
    let repeated_legacy = fs::read(&state).unwrap();
    FileRepositoryFitLedger::open_existing(&fixture.store.root, fixture.store.store_id()).unwrap();
    assert_eq!(fs::read(&state).unwrap(), repeated_legacy);

    let bytes = String::from_utf8(repeated_legacy).unwrap();
    fs::write(
        &state,
        bytes.replacen("\"generation\":4", "\"generation\":5", 1),
    )
    .unwrap();
    assert!(matches!(
        FileRepositoryFitLedger::open_existing(&fixture.store.root, fixture.store.store_id()),
        Err(ref error) if error.id() == LedgerErrorId::Tampered
    ));
}

#[test]
pub(crate) fn authenticated_legacy_durable_identity_drift_still_fails_closed() {
    let fixture = Fixture::new("legacy-durable-identity-drift");
    FileRepositoryFitLedger::open_or_initialize(&fixture.store.root, fixture.store.store_id())
        .unwrap();
    let current_inode = fs::metadata(&fixture.store.root).unwrap().ino();
    simulate_durable_identity_drift_for_test(&fixture.store.root, current_inode + 1).unwrap();

    assert!(matches!(
        FileRepositoryFitLedger::open_existing(&fixture.store.root, fixture.store.store_id()),
        Err(ref error) if error.id() == LedgerErrorId::Tampered
    ));
}
