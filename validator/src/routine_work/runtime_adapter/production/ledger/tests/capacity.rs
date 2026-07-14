use super::*;

#[test]
fn protocol_effect_and_consumed_grant_capacity_refuse_real_next_reservation_transactionally() {
    let protocol_root = TestRoot::new("protocol-capacity");
    let protocol_ledger = FileAuthorityLedger::open_or_initialize(protocol_root.path()).unwrap();
    let (records, consumed) = FileAuthorityLedger::test_capacity_limits();
    protocol_ledger
        .test_seed_capacity(records, records)
        .unwrap();
    let exact = protocol_root.state();
    let error = match protocol_ledger.reserve(reservation("protocol-over-capacity")) {
        Ok(_) => panic!("protocol/effect capacity accepted one more reservation"),
        Err(error) => error,
    };
    assert_eq!(
        error.cause(),
        "routine-production-authority-capacity-exhausted"
    );
    assert_eq!(protocol_root.state(), exact);

    let consumed_root = TestRoot::new("consumed-capacity");
    let consumed_ledger = FileAuthorityLedger::open_or_initialize(consumed_root.path()).unwrap();
    consumed_ledger.test_seed_capacity(0, consumed).unwrap();
    let exact = consumed_root.state();
    let error = match consumed_ledger.reserve(reservation("consumed-over-capacity")) {
        Ok(_) => panic!("consumed-grant capacity accepted one more reservation"),
        Err(error) => error,
    };
    assert_eq!(
        error.cause(),
        "routine-production-authority-capacity-exhausted"
    );
    assert_eq!(consumed_root.state(), exact);
}
