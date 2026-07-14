use super::*;

#[test]
pub(crate) fn stale_target_and_invalid_clock_refuse_before_authority_store_write() {
    let stale = Fixture::new("stale-before-store");
    let context = stale.context();
    let prepared = stale.prepared(&context);
    let recovery_intent = prepare_recovery_intent(&context, &prepared).unwrap();
    stale.write(CANONICAL_TEMPLATES[0].target_path, b"attacker bytes\n");
    let before = snapshot(&stale.root);
    let refused = execute_prepared_apply(
        &context,
        prepared,
        recovery_intent,
        &TestClock::new([10, 11, 12]),
        &stale.store,
        nonce("stale-before-store"),
    );
    assert_eq!(refused.status(), "refused");
    assert_eq!(stale.store_names(), Vec::<String>::new());
    assert_eq!(snapshot(&stale.root), before);

    let expired = Fixture::new("clock-before-store");
    let context = expired.context();
    let prepared = expired.prepared(&context);
    let recovery_intent = prepare_recovery_intent(&context, &prepared).unwrap();
    let before = snapshot(&expired.root);
    let result = execute_prepared_apply(
        &context,
        prepared,
        recovery_intent,
        &TestClock::new([100, 99]),
        &expired.store,
        nonce("clock-before-store"),
    );
    assert_eq!(result.error_id(), Some(AdapterErrorId::ApplyPermitExpired));
    assert_eq!(expired.store_names(), Vec::<String>::new());
    assert_eq!(snapshot(&expired.root), before);
}

#[test]
pub(crate) fn protected_store_drift_after_reservation_refuses_before_workspace_effect() {
    let fixture = Fixture::new("protected-store-drift-before-effect");
    let context = fixture.context();
    let prepared = fixture.prepared(&context);
    let recovery_intent = prepare_recovery_intent(&context, &prepared).unwrap();
    let target_before = snapshot(&fixture.root);
    let store = RevalidationStore::new(&fixture.store, 2);

    let outcome = execute_prepared_apply(
        &context,
        prepared,
        recovery_intent,
        &TestClock::new([10, 11, 12]),
        &store,
        nonce("protected-store-drift-before-effect"),
    );

    assert_eq!(store.calls.load(Ordering::SeqCst), 3);
    assert_eq!(outcome.status(), "refused");
    assert_eq!(outcome.error_id(), Some(AdapterErrorId::ApplyPermitInvalid));
    assert!(!outcome.effect_started());
    assert_eq!(snapshot(&fixture.root), target_before);
    assert!(
        String::from_utf8(fs::read(fixture.store.root.join("authority-ledger.json")).unwrap())
            .unwrap()
            .contains("rejected")
    );
}

#[test]
pub(crate) fn managed_ancestor_replacement_invalidates_recovery_intent_before_store_open() {
    let fixture = Fixture::new("ancestor-intent-before-store");
    let ancestor = create_existing_codex_ancestor(&fixture);
    let context = fixture.context();
    let prepared = fixture.prepared(&context);
    let recovery_intent = prepare_recovery_intent(&context, &prepared).unwrap();
    replace_directory_preserving_children(
        &ancestor,
        &fixture.container.join("original-pre-reservation-codex"),
    );
    let before = snapshot(&fixture.root);

    let outcome = execute_prepared_apply(
        &context,
        prepared,
        recovery_intent,
        &TestClock::new([10, 11, 12]),
        &fixture.store,
        nonce("ancestor-intent-before-store"),
    );

    assert_eq!(outcome.status(), "refused");
    assert_eq!(outcome.error_id(), Some(AdapterErrorId::ApplyPermitInvalid));
    assert_eq!(fixture.store_names(), Vec::<String>::new());
    assert_eq!(snapshot(&fixture.root), before);
}

#[test]
pub(crate) fn post_reservation_target_mutation_is_rejected_without_authority_mutation() {
    let fixture = Fixture::new("post-reservation-target-race");
    let context = fixture.context();
    let prepared = fixture.prepared(&context);
    let target = fixture.root.join(CANONICAL_TEMPLATES[0].target_path);
    after_reservation_for_test(move || {
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, b"attacker raced after reservation\n").unwrap();
    });
    let outcome = execute(&fixture, &context, prepared, "post-reservation-race");
    assert_eq!(outcome.status(), "refused");
    assert_eq!(outcome.error_id(), Some(AdapterErrorId::StalePlan));
    assert!(!outcome.effect_started());
    assert_eq!(
        fs::read(fixture.root.join(CANONICAL_TEMPLATES[0].target_path)).unwrap(),
        b"attacker raced after reservation\n"
    );
    for row in &CANONICAL_TEMPLATES[1..] {
        assert!(!fixture.root.join(row.target_path).exists());
    }
    assert!(
        String::from_utf8(fs::read(fixture.store.root.join("authority-ledger.json")).unwrap())
            .unwrap()
            .contains("rejected")
    );
}

#[test]
pub(crate) fn symlink_hardlink_and_special_target_substitution_refuse_before_store() {
    for kind in ["symlink", "hardlink", "fifo"] {
        let fixture = Fixture::new(&format!("target-{kind}"));
        let context = fixture.context();
        let prepared = fixture.prepared(&context);
        let recovery_intent = prepare_recovery_intent(&context, &prepared).unwrap();
        let target = fixture.root.join(CANONICAL_TEMPLATES[0].target_path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        match kind {
            "symlink" => symlink("../outside", &target).unwrap(),
            "hardlink" => {
                let source = fixture.root.join("hardlink-source");
                fs::write(&source, b"hardlink").unwrap();
                fs::hard_link(source, &target).unwrap();
            }
            "fifo" => {
                let path = std::ffi::CString::new(target.as_os_str().as_encoded_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
            }
            _ => unreachable!(),
        }
        let outcome = execute_prepared_apply(
            &context,
            prepared,
            recovery_intent,
            &TestClock::new([10, 11, 12]),
            &fixture.store,
            nonce(kind),
        );
        assert_eq!(outcome.status(), "refused");
        assert_eq!(fixture.store_names(), Vec::<String>::new());
    }
}

#[test]
pub(crate) fn canonical_outcome_redacts_nonce_path_secret_and_backend_canaries() {
    let fixture = Fixture::new("redaction");
    let context = fixture.context();
    let canary = "zzq7V5-repository-fit-production-nonce-secret";
    let prepared = fixture.prepared(&context);
    let recovery_intent = prepare_recovery_intent(&context, &prepared).unwrap();
    let outcome = execute_prepared_apply(
        &context,
        prepared,
        recovery_intent,
        &TestClock::new([10, 11, 12]),
        &fixture.store,
        RepositoryFitApplyNonce::new(canary.repeat(2).into_bytes()).unwrap(),
    );
    assert_eq!(outcome.status(), "applied");
    let machine = String::from_utf8(outcome.to_machine_bytes().unwrap()).unwrap();
    assert!(!machine.contains(canary));
    assert!(!machine.contains(fixture.root.to_str().unwrap()));
    assert!(!machine.contains(fixture.store.root.to_str().unwrap()));
    assert!(!machine.contains("raw-backend-output-canary"));
    assert!(machine.len() <= 16 * 1024);
}
