use super::*;

#[test]
pub(crate) fn live_effect_owner_holds_process_lock_through_mutation_and_terminal() {
    let fixture = Fixture::new("effect-owner-lock-race");
    let owner_ready = fixture.container.join("effect-owner-ready");
    let owner_release = fixture.container.join("effect-owner-release");
    let owner_result = fixture.container.join("effect-owner-result.json");
    let recovery_ready = fixture.container.join("effect-recovery-ready");
    let recovery_release = fixture.container.join("effect-recovery-unused-release");
    let recovery_result = fixture.container.join("effect-recovery-result.json");

    let owner = authority_scenario_command(
        &fixture,
        "winner-effect",
        "effect-owner-lock",
        &owner_result,
        &owner_ready,
        &owner_release,
    )
    .spawn()
    .unwrap();
    let owner = ChildGuard::new(owner);
    wait_for_path(&owner_ready);

    let recovery = authority_scenario_command(
        &fixture,
        "recover-expired-signaled",
        "effect-owner-lock",
        &recovery_result,
        &recovery_ready,
        &recovery_release,
    )
    .spawn()
    .unwrap();
    let mut recovery = ChildGuard::new(recovery);
    wait_for_path(&recovery_ready);
    let blocked_until = Instant::now() + Duration::from_millis(100);
    while Instant::now() < blocked_until {
        assert!(
            recovery.try_wait().is_none(),
            "recovery escaped while the live effect owner held the exact lock"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!recovery_result.exists());

    fs::write(&owner_release, b"release\n").unwrap();
    let owner_output = owner.wait_with_output();
    let recovery_output = recovery.wait_with_output();
    assert!(owner_output.status.success(), "{owner_output:?}");
    assert!(recovery_output.status.success(), "{recovery_output:?}");

    let owner = result_value(&owner_result);
    assert_eq!(owner["status"], "applied");
    assert_eq!(owner["ledger_state"], "committed");
    let recovery = result_value(&recovery_result);
    assert_eq!(recovery["status"], "refused");
    assert_eq!(recovery["adapter_error_id"], "apply_permit_replayed");
    assert_eq!(recovery["ledger_state"], "committed");
    assert_eq!(recovery["effect"], "none");

    let terminal = fs::read_to_string(fixture.store.root.join("authority-ledger.json")).unwrap();
    assert_eq!(terminal.matches("\"state\":\"reserved\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"effect_started\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"committed\"").count(), 1);
    assert!(!terminal.contains("interrupted"));
    assert!(!terminal.contains("ambiguous"));
}

#[test]
pub(crate) fn two_processes_racing_one_nonce_and_semantic_effect_have_one_ledger_winner() {
    let fixture = Fixture::new("process-race");
    let _ledger =
        FileRepositoryFitLedger::open_or_initialize(&fixture.store.root, fixture.store.store_id())
            .unwrap();
    let barrier = fixture.container.join("race-start");
    let reached = Arc::new(Barrier::new(3));
    let mut children = Vec::new();
    for index in 0..2 {
        let root = fixture.store.root.clone();
        let id = fixture.store.id.clone();
        let barrier = barrier.clone();
        let reached = Arc::clone(&reached);
        children.push(std::thread::spawn(move || {
            reached.wait();
            if index == 0 {
                fs::write(&barrier, b"go").unwrap();
            } else {
                while !barrier.exists() {
                    std::thread::yield_now();
                }
            }
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "repository_fit::product_adapter::tests::production_authority::subprocess_reservation_helper",
                    "--nocapture",
                ])
                .env("HUL_FIT_AUTHORITY_SUBPROCESS", "1")
                .env("HUL_FIT_AUTHORITY_STORE", root)
                .env("HUL_FIT_AUTHORITY_STORE_ID", id)
                .output()
                .unwrap()
        }));
    }
    reached.wait();
    let outputs = children
        .into_iter()
        .map(|child| child.join().unwrap())
        .collect::<Vec<_>>();
    for output in &outputs {
        assert!(output.status.success(), "{output:?}");
    }
    let joined = outputs
        .iter()
        .map(|output| String::from_utf8_lossy(&output.stdout))
        .collect::<String>();
    assert_eq!(joined.matches("RESERVATION-WINNER").count(), 1, "{joined}");
    assert_eq!(
        joined.matches("RESERVATION-EXISTING").count(),
        1,
        "{joined}"
    );
}

#[test]
pub(crate) fn opener_waits_on_the_exact_lock_while_a_legitimate_atomic_publication_is_in_flight() {
    let fixture = Fixture::new("atomic-publication-open-race");
    let writer_ledger =
        FileRepositoryFitLedger::open_or_initialize(&fixture.store.root, fixture.store.store_id())
            .unwrap();
    let (publish_reached_tx, publish_reached_rx) = mpsc::sync_channel(0);
    let (release_publish_tx, release_publish_rx) = mpsc::sync_channel(0);
    let writer = std::thread::spawn(move || {
        before_atomic_publish_for_test(move || {
            publish_reached_tx.send(()).unwrap();
            release_publish_rx.recv().unwrap();
        });
        match writer_ledger.reserve(reservation('1')).unwrap() {
            ReservationDecision::Acquired(_) => "winner",
            ReservationDecision::Existing(_) => "existing",
        }
    });
    publish_reached_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("writer must pause with its temporary state file fully written under the lock");

    let opener_root = fixture.store.root.clone();
    let opener_store_id = fixture.store.id.clone();
    let (opener_reached_tx, opener_reached_rx) = mpsc::sync_channel(0);
    let opener = std::thread::spawn(move || {
        before_lock_acquire_for_test(move || opener_reached_tx.send(()).unwrap());
        let ledger = FileRepositoryFitLedger::open_or_initialize(&opener_root, &opener_store_id)
            .expect("an in-flight legitimate publication is not store tampering");
        match ledger.reserve(reservation('1')).unwrap() {
            ReservationDecision::Acquired(_) => "winner",
            ReservationDecision::Existing(existing) => {
                assert_eq!(existing.state(), RepositoryFitLedgerState::Reserved);
                "existing"
            }
        }
    });
    opener_reached_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("opener must bind the exact lock without enumerating transient state");
    release_publish_tx.send(()).unwrap();

    assert_eq!(writer.join().unwrap(), "winner");
    assert_eq!(opener.join().unwrap(), "existing");
}
