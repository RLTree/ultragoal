#[cfg(unix)]
#[test]
fn authorized_effect_enforces_exact_live_executable_identity() {
    let first_fixture = test_fixture("authorized-first", b"#!/bin/sh\nexit 0\n");
    let second_fixture = test_fixture("authorized-second", b"#!/bin/sh\nexit 0\n");
    let first = first_fixture.selected.duplicate().unwrap();
    let second = second_fixture.selected.duplicate().unwrap();
    let plan = HostCommandPlan::personal_install(&package(), "local-harness").unwrap();
    let authority =
        HostEffectAuthority::generate("root-actor".to_owned(), "host-ledger".to_owned()).unwrap();
    let (permit, reservation) = authority.issue(permit_binding(&plan, &first)).unwrap();
    authority.verify(&permit, 1_500).unwrap();

    assert_eq!(
        AuthorizedHostEffect::new(permit, in_flight_record(reservation), second, plan)
            .err()
            .unwrap()
            .id(),
        HostEffectLedgerErrorId::InvalidRecord
    );

    let pinned = first_fixture.selected.duplicate().unwrap();
    let plan = HostCommandPlan::personal_install(&package(), "local-harness").unwrap();
    let (permit, reservation) = authority.issue(permit_binding(&plan, &pinned)).unwrap();
    authority.verify(&permit, 1_500).unwrap();
    let authorized =
        AuthorizedHostEffect::new(permit, in_flight_record(reservation), pinned, plan).unwrap();
    authorized.executable().revalidate().unwrap();

    let changed_fixture = test_fixture("authorized-changed", b"#!/bin/sh\nexit 0\n");
    let changed = changed_fixture.selected.duplicate().unwrap();
    let plan = HostCommandPlan::personal_install(&package(), "local-harness").unwrap();
    let (permit, reservation) = authority.issue(permit_binding(&plan, &changed)).unwrap();
    authority.verify(&permit, 1_500).unwrap();
    let mut file = OpenOptions::new()
        .write(true)
        .open(&changed_fixture.path)
        .unwrap();
    file.write_all(b"#!/bin/sh\nexit 9\n").unwrap();
    file.sync_all().unwrap();
    assert_eq!(
        AuthorizedHostEffect::new(permit, in_flight_record(reservation), changed, plan)
            .err()
            .unwrap()
            .id(),
        HostEffectLedgerErrorId::Tampered
    );
}
