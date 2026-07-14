use super::*;

#[test]
pub(crate) fn subprocess_reservation_helper() {
    if std::env::var_os("HUL_FIT_AUTHORITY_SUBPROCESS").is_none() {
        return;
    }
    let root = PathBuf::from(std::env::var_os("HUL_FIT_AUTHORITY_STORE").unwrap());
    let id = std::env::var("HUL_FIT_AUTHORITY_STORE_ID").unwrap();
    let ledger = FileRepositoryFitLedger::open_or_initialize(&root, &id).unwrap();
    match ledger.reserve(reservation('1')).unwrap() {
        ReservationDecision::Acquired(_) => println!("RESERVATION-WINNER"),
        ReservationDecision::Existing(existing) => {
            assert_eq!(existing.state(), RepositoryFitLedgerState::Reserved);
            assert_eq!(existing.terminal_sha256(), None);
            println!("RESERVATION-EXISTING");
        }
    }
}

#[test]
pub(crate) fn subprocess_authority_scenario_helper() {
    let Some(role) = std::env::var_os("HUL_FIT_AUTHORITY_SCENARIO") else {
        return;
    };
    let role = role.to_string_lossy().into_owned();
    let root = PathBuf::from(std::env::var_os("HUL_FIT_AUTHORITY_REPO").unwrap());
    let store = TestStore {
        root: PathBuf::from(std::env::var_os("HUL_FIT_AUTHORITY_STORE").unwrap()),
        id: std::env::var("HUL_FIT_AUTHORITY_STORE_ID").unwrap(),
    };
    let nonce_label = std::env::var("HUL_FIT_AUTHORITY_NONCE_LABEL").unwrap();
    let intent_path = PathBuf::from(std::env::var_os("HUL_FIT_AUTHORITY_INTENT").unwrap());
    let result = PathBuf::from(std::env::var_os("HUL_FIT_AUTHORITY_RESULT").unwrap());
    let ready = PathBuf::from(std::env::var_os("HUL_FIT_AUTHORITY_READY").unwrap());
    let release = PathBuf::from(std::env::var_os("HUL_FIT_AUTHORITY_RELEASE").unwrap());
    let context = LiveContext::build(BuildRequest::new(&root)).unwrap();

    if matches!(
        role.as_str(),
        "recover-active" | "recover-expired" | "recover-expired-signaled"
    ) {
        if role == "recover-expired-signaled" {
            fs::write(&ready, b"recovery-started\n").unwrap();
        }
        let intent_bytes = fs::read(&intent_path).unwrap();
        let tick = if role == "recover-active" { 22 } else { 73 };
        let outcome = recover_prepared_apply(
            &context,
            &intent_bytes,
            &TestClock::new([tick]),
            &store,
            nonce(&nonce_label),
        );
        fs::write(result, outcome.to_machine_bytes().unwrap()).unwrap();
        return;
    }

    let plan = plan_target(&context).unwrap();
    let prepared = prepare_apply_request(
        &context,
        &plan.to_machine_bytes().unwrap(),
        plan.plan_sha256(),
    )
    .unwrap();
    let recovery_intent = prepare_recovery_intent(&context, &prepared).unwrap();
    let intent_bytes = recovery_intent.to_machine_bytes();
    if intent_path.exists() {
        assert_eq!(fs::read(&intent_path).unwrap(), intent_bytes);
    } else {
        fs::write(&intent_path, &intent_bytes).unwrap();
    }

    match role.as_str() {
        "winner" => after_reservation_for_test(move || {
            fs::write(&ready, b"reserved\n").unwrap();
            wait_for_path(&release);
        }),
        "winner-effect" => after_effect_start_before_apply_for_test(move || {
            fs::write(&ready, b"effect-started\n").unwrap();
            wait_for_path(&release);
        }),
        "contender" => {}
        "expired-execute" => {}
        "crash-before" => after_reservation_for_test(|| unsafe { libc::_exit(86) }),
        "crash-after-start" => {
            after_effect_start_before_apply_for_test(|| unsafe { libc::_exit(89) })
        }
        "crash-after" => after_effect_before_terminal_for_test(|| unsafe { libc::_exit(88) }),
        other => panic!("unknown authority subprocess role: {other}"),
    }

    let ticks = match role.as_str() {
        "contender" => [20, 21, 22],
        "expired-execute" => [71, 72, 73],
        _ => [10, 11, 12],
    };
    let outcome = execute_prepared_apply(
        &context,
        prepared,
        recovery_intent,
        &TestClock::new(ticks),
        &store,
        nonce(&nonce_label),
    );
    fs::write(result, outcome.to_machine_bytes().unwrap()).unwrap();
}

#[test]
pub(crate) fn nonce_and_semantic_replay_and_active_target_lease_fail_closed() {
    let fixture = Fixture::new("replay-lease");
    let ledger =
        FileRepositoryFitLedger::open_or_initialize(&fixture.store.root, fixture.store.store_id())
            .unwrap();
    assert!(matches!(
        ledger.reserve(reservation('1')).unwrap(),
        ReservationDecision::Acquired(_)
    ));
    assert!(matches!(
        ledger.reserve(reservation('1')).unwrap(),
        ReservationDecision::Existing(_)
    ));
    let first = reservation('1');
    let same_target = ReservationRequest {
        binding_sha256: &fixed_digest('a'),
        semantic_effect_id: &fixed_digest('b'),
        target_scope_id: first.target_scope_id,
        permit_id: &fixed_digest('c'),
        nonce_sha256: &fixed_digest('d'),
        recovery_intent_sha256: first.recovery_intent_sha256,
        issued_tick: 10,
        expires_tick: 20,
        recovery: first.recovery,
    };
    assert!(matches!(
        ledger.reserve(same_target),
        Err(ref error) if error.id() == LedgerErrorId::ActiveLease
    ));
    let same_nonce_other_effect = ReservationRequest {
        binding_sha256: &fixed_digest('a'),
        semantic_effect_id: &fixed_digest('b'),
        target_scope_id: &fixed_digest('c'),
        permit_id: &fixed_digest('d'),
        nonce_sha256: first.nonce_sha256,
        recovery_intent_sha256: first.recovery_intent_sha256,
        issued_tick: 10,
        expires_tick: 20,
        recovery: first.recovery,
    };
    assert!(matches!(
        ledger.reserve(same_nonce_other_effect),
        Err(ref error) if error.id() == LedgerErrorId::Replay
    ));

    let mut mismatched_recovery = first.recovery.clone();
    mismatched_recovery.rows[0].post_sha256 = fixed_digest('a');
    let mismatched_intent = ReservationRequest {
        binding_sha256: &fixed_digest('a'),
        semantic_effect_id: &fixed_digest('b'),
        target_scope_id: &fixed_digest('c'),
        permit_id: &fixed_digest('d'),
        nonce_sha256: &fixed_digest('e'),
        recovery_intent_sha256: first.recovery_intent_sha256,
        issued_tick: 10,
        expires_tick: 20,
        recovery: &mismatched_recovery,
    };
    assert!(matches!(
        ledger.reserve(mismatched_intent),
        Err(ref error) if error.id() == LedgerErrorId::InvalidTransition
    ));
}
