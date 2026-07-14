use super::*;

#[test]
pub(crate) fn same_session_stale_head_replay_and_terminal_substitution_fail_closed() {
    let fixture = Fixture::new("stale-head");
    let ledger =
        FileRepositoryFitLedger::open_or_initialize(&fixture.store.root, fixture.store.store_id())
            .unwrap();
    let token = match ledger.reserve(reservation('1')).unwrap() {
        ReservationDecision::Acquired(token) => token,
        ReservationDecision::Existing(_) => unreachable!(),
    };
    let reserved = ledger.snapshot_for_test().unwrap();
    ledger
        .terminal(
            token,
            RepositoryFitLedgerState::Rejected,
            &fixed_digest('9'),
            Some(AdapterErrorId::ApplyPermitInvalid),
            11,
        )
        .unwrap();
    fs::write(fixture.store.root.join("authority-ledger.json"), reserved).unwrap();
    assert!(matches!(
        ledger.reserve(reservation('a')),
        Err(ref error) if error.id() == LedgerErrorId::Tampered
    ));

    let other = Fixture::new("terminal-substitution");
    let other_ledger =
        FileRepositoryFitLedger::open_or_initialize(&other.store.root, other.store.store_id())
            .unwrap();
    let token = match other_ledger.reserve(reservation('1')).unwrap() {
        ReservationDecision::Acquired(token) => token,
        ReservationDecision::Existing(_) => unreachable!(),
    };
    other_ledger
        .terminal(
            token,
            RepositoryFitLedgerState::Rejected,
            &fixed_digest('9'),
            Some(AdapterErrorId::ApplyPermitInvalid),
            11,
        )
        .unwrap();
    let state = other.store.root.join("authority-ledger.json");
    let bytes = String::from_utf8(fs::read(&state).unwrap()).unwrap();
    fs::write(&state, bytes.replacen("rejected", "committed", 1)).unwrap();
    assert!(matches!(
        other_ledger.reserve(reservation('a')),
        Err(ref error) if error.id() == LedgerErrorId::Tampered
    ));
}

#[test]
pub(crate) fn two_process_execute_through_terminal_allows_only_the_reserved_owner_to_mutate() {
    let fixture = Fixture::new("full-process-owner-race");
    let before = snapshot(&fixture.root);
    let ready = fixture.container.join("winner-reserved");
    let release = fixture.container.join("release-winner");
    let winner_result = fixture.container.join("winner-result.json");
    let contender_result = fixture.container.join("contender-result.json");

    let winner = authority_scenario_command(
        &fixture,
        "winner",
        "full-process-owner",
        &winner_result,
        &ready,
        &release,
    )
    .spawn()
    .unwrap();
    let winner = ChildGuard::new(winner);
    wait_for_path(&ready);

    let contender_output = authority_scenario_command(
        &fixture,
        "contender",
        "full-process-owner",
        &contender_result,
        &ready,
        &release,
    )
    .output()
    .unwrap();
    let before_release = snapshot(&fixture.root);
    let reserved_ledger = fs::read_to_string(fixture.store.root.join("authority-ledger.json"))
        .expect("the winner reservation is durable");
    fs::write(&release, b"release\n").unwrap();
    let winner_output = winner.wait_with_output();

    assert!(contender_output.status.success(), "{contender_output:?}");
    assert!(winner_output.status.success(), "{winner_output:?}");
    assert_eq!(before_release, before, "the contender changed the target");
    assert!(reserved_ledger.contains("reserved"));
    assert!(!reserved_ledger.contains("effect_started"));
    assert!(!reserved_ledger.contains("committed"));

    let contender = result_value(&contender_result);
    assert_eq!(contender["status"], "refused");
    assert_eq!(contender["adapter_error_id"], "apply_lease_invalid");
    assert_eq!(contender["ledger_state"], "reserved");
    assert_eq!(contender["effect_started"], false);
    assert_eq!(contender["effect"], "none");

    let winner = result_value(&winner_result);
    assert_eq!(winner["status"], "applied");
    assert_eq!(winner["ledger_state"], "committed");
    assert_eq!(winner["effect_started"], true);
    assert_eq!(winner["effect"], "workspace_write");
    assert_eq!(
        winner["apply_outcome"]["mutation_count"],
        CANONICAL_TEMPLATES.len()
    );
    for row in CANONICAL_TEMPLATES {
        assert_eq!(
            fs::read(fixture.root.join(row.target_path)).unwrap(),
            row.bytes
        );
    }
    let terminal = fs::read_to_string(fixture.store.root.join("authority-ledger.json")).unwrap();
    assert_eq!(terminal.matches("\"state\":\"reserved\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"effect_started\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"committed\"").count(), 1);
    assert!(!terminal.contains("interrupted"));
    assert!(!terminal.contains("ambiguous"));
}
