use super::*;

#[test]
pub(crate) fn missing_recovery_ledger_refuses_without_initializing_authority_or_writing_target() {
    let fixture = Fixture::new("missing-recovery-ledger");
    let context = fixture.context();
    let intent = prepare_recovery_intent(&context, &fixture.prepared(&context))
        .unwrap()
        .to_machine_bytes();
    let before_target = snapshot(&fixture.root);
    let before_status = git_status(&fixture.root);
    let before_store = fixture.store_names();
    assert!(before_store.is_empty());

    let outcome = recover_prepared_apply(
        &context,
        &intent,
        &TestClock::new([73]),
        &fixture.store,
        nonce("missing-recovery-ledger-owner"),
    );

    assert_eq!(outcome.status(), "refused");
    assert_eq!(
        outcome.error_id(),
        Some(AdapterErrorId::ApplyPermitReplayed)
    );
    assert_eq!(fixture.store_names(), before_store);
    assert_eq!(snapshot(&fixture.root), before_target);
    assert_eq!(git_status(&fixture.root), before_status);
}

#[test]
pub(crate) fn recovery_existing_only_open_rejects_whole_authority_substitution_without_reinitializing()
 {
    let fixture = Fixture::new("recovery-existing-only-substitution");
    let intent_path =
        run_authority_crash(&fixture, "crash-before", "recovery-existing-only-owner", 86);
    let intent = fs::read(intent_path).unwrap();
    let before_target = snapshot(&fixture.root);
    let before_status = git_status(&fixture.root);
    let repository_root = fixture.root.clone();
    let store_root = fixture.store.root.clone();
    let store_id = fixture.store.id.clone();
    let (open_reached_tx, open_reached_rx) = mpsc::sync_channel(0);
    let (release_open_tx, release_open_rx) = mpsc::sync_channel(0);
    let recovery = std::thread::spawn(move || {
        before_existing_open_for_test(move || {
            open_reached_tx.send(()).unwrap();
            release_open_rx.recv().unwrap();
        });
        let context = LiveContext::build(BuildRequest::new(&repository_root)).unwrap();
        let store = TestStore {
            root: store_root,
            id: store_id,
        };
        let outcome = recover_prepared_apply(
            &context,
            &intent,
            &TestClock::new([73]),
            &store,
            nonce("recovery-existing-only-owner"),
        );
        (outcome.status().to_owned(), outcome.error_id())
    });
    open_reached_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("recovery must pause after binding the store root and observing the named lock");

    let quarantine = fixture.container.join("removed-authority");
    fs::create_dir(&quarantine).unwrap();
    let original_names = fixture.store_names();
    assert_eq!(
        original_names,
        ["authority-ledger.json", "authority.key", "authority.lock"]
    );
    for name in &original_names {
        fs::rename(fixture.store.root.join(name), quarantine.join(name)).unwrap();
    }
    let quarantined = original_names
        .iter()
        .map(|name| (name.clone(), fs::read(quarantine.join(name)).unwrap()))
        .collect::<BTreeMap<_, _>>();
    assert!(fixture.store_names().is_empty());
    release_open_tx.send(()).unwrap();

    let (status, error_id) = recovery.join().unwrap();
    assert_eq!(status, "refused");
    assert_eq!(error_id, Some(AdapterErrorId::ApplyOutcomeInvalid));
    assert!(fixture.store_names().is_empty());
    for (name, bytes) in quarantined {
        assert_eq!(fs::read(quarantine.join(name)).unwrap(), bytes);
    }
    assert_eq!(snapshot(&fixture.root), before_target);
    assert_eq!(git_status(&fixture.root), before_status);
}

#[test]
pub(crate) fn abrupt_process_exit_before_effect_recovers_only_after_expiry_without_target_write() {
    let fixture = Fixture::new("process-crash-before-effect");
    let before = snapshot(&fixture.root);
    let _intent = run_authority_crash(&fixture, "crash-before", "crash-before-owner", 86);

    assert_eq!(snapshot(&fixture.root), before);
    let reserved = fs::read_to_string(fixture.store.root.join("authority-ledger.json")).unwrap();
    assert_eq!(reserved.matches("\"state\":\"reserved\"").count(), 1);
    assert!(!reserved.contains("effect_started"));
    assert!(!reserved.contains("committed"));

    let active = run_authority_scenario(&fixture, "recover-active", "crash-before-owner");
    assert_eq!(active["status"], "refused");
    assert_eq!(active["adapter_error_id"], "apply_lease_invalid");
    assert_eq!(active["ledger_state"], "reserved");
    assert_eq!(active["effect_started"], false);
    assert_eq!(snapshot(&fixture.root), before);

    let ordinary = run_authority_scenario(&fixture, "expired-execute", "crash-before-owner");
    assert_eq!(ordinary["status"], "refused");
    assert_eq!(ordinary["adapter_error_id"], "apply_lease_invalid");
    assert_eq!(ordinary["ledger_state"], "reserved");
    assert_eq!(ordinary["effect_started"], false);
    assert_eq!(snapshot(&fixture.root), before);

    let expired = run_authority_scenario(&fixture, "recover-expired", "crash-before-owner");
    assert_eq!(expired["status"], "interrupted");
    assert_eq!(expired["adapter_error_id"], "apply_outcome_ambiguous");
    assert_eq!(expired["ledger_state"], "interrupted");
    assert_eq!(expired["effect_started"], false);
    assert_eq!(expired["effect"], "none");
    assert_eq!(snapshot(&fixture.root), before);

    let terminal = fs::read_to_string(fixture.store.root.join("authority-ledger.json")).unwrap();
    assert_eq!(terminal.matches("\"state\":\"reserved\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"interrupted\"").count(), 1);
    assert!(!terminal.contains("effect_started"));
    assert!(!terminal.contains("committed"));
}

#[test]
pub(crate) fn abrupt_process_exit_after_effect_with_same_root_binding_recovers_exact_postimage_as_committed()
 {
    let fixture = Fixture::new("process-crash-after-effect");
    let prepared_root_binding = observed_root_binding(&fixture.root);
    let _intent = run_authority_crash(&fixture, "crash-after", "crash-after-owner", 88);
    assert_eq!(observed_root_binding(&fixture.root), prepared_root_binding);

    for row in CANONICAL_TEMPLATES {
        assert_eq!(
            fs::read(fixture.root.join(row.target_path)).unwrap(),
            row.bytes
        );
    }
    let started = fs::read_to_string(fixture.store.root.join("authority-ledger.json")).unwrap();
    assert_eq!(started.matches("\"state\":\"reserved\"").count(), 1);
    assert_eq!(started.matches("\"state\":\"effect_started\"").count(), 1);
    assert!(!started.contains("committed"));

    let recovered = run_authority_scenario(&fixture, "recover-expired", "crash-after-owner");
    assert_eq!(recovered["status"], "recovered");
    assert_eq!(recovered["adapter_error_id"], serde_json::Value::Null);
    assert_eq!(recovered["effect_started"], true);
    assert_eq!(recovered["effect"], "none");
    assert_eq!(recovered["ledger_state"], "committed");

    let terminal = fs::read_to_string(fixture.store.root.join("authority-ledger.json")).unwrap();
    assert_eq!(terminal.matches("\"state\":\"reserved\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"effect_started\"").count(), 1);
    assert_eq!(terminal.matches("\"state\":\"committed\"").count(), 1);
    assert!(!terminal.contains("interrupted"));
    assert!(!terminal.contains("ambiguous"));
}
