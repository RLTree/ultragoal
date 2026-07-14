use super::*;

#[test]
pub(crate) fn failed_terminal_validation_retains_recovery_authority_until_reconciled() {
    let fixture = Fixture::new("terminal-validation-recovery-retained");
    let context = fixture.context();
    let prepared = fixture.prepared(&context);
    let recovery_intent = prepare_recovery_intent(&context, &prepared).unwrap();
    let intent_bytes = recovery_intent.to_machine_bytes();
    let store_root = fixture.store.root.clone();
    after_effect_before_terminal_for_test(move || {
        before_atomic_publish_for_test(move || {
            fs::set_permissions(&store_root, fs::Permissions::from_mode(0o777)).unwrap();
        });
    });

    let outcome = execute_prepared_apply(
        &context,
        prepared,
        recovery_intent,
        &TestClock::new([10, 11, 12]),
        &fixture.store,
        nonce("terminal-validation-recovery-retained"),
    );

    assert_eq!(outcome.status(), "ambiguous");
    assert_eq!(
        outcome.error_id(),
        Some(AdapterErrorId::ApplyOutcomeAmbiguous)
    );
    assert!(outcome.effect_started());
    assert!(outcome.recovery_required());
    fs::set_permissions(&fixture.store.root, fs::Permissions::from_mode(0o700)).unwrap();

    let recovered = recover_prepared_apply(
        &context,
        &intent_bytes,
        &TestClock::new([73]),
        &fixture.store,
        nonce("terminal-validation-recovery-retained"),
    );
    assert_eq!(
        recovered.error_id(),
        Some(AdapterErrorId::ApplyPermitReplayed)
    );
    assert!(!recovered.recovery_required());
    for row in CANONICAL_TEMPLATES {
        assert_eq!(
            fs::read(fixture.root.join(row.target_path)).unwrap(),
            row.bytes
        );
    }
}

#[test]
pub(crate) fn partial_dirty_target_preserves_user_bytes_and_applies_only_plan() {
    let fixture = Fixture::new("partial-dirty");
    fixture.write("USER-NOTES.txt", b"keep these user bytes\n");
    fixture.write_template(CANONICAL_TEMPLATES[0].target_path);
    let before_user = fs::read(fixture.root.join("USER-NOTES.txt")).unwrap();
    let context = fixture.context();
    let outcome = execute(
        &fixture,
        &context,
        fixture.prepared(&context),
        "partial-dirty",
    );
    assert_eq!(outcome.status(), "applied");
    assert_eq!(
        fs::read(fixture.root.join("USER-NOTES.txt")).unwrap(),
        before_user
    );
    for row in CANONICAL_TEMPLATES {
        assert_eq!(
            fs::read(fixture.root.join(row.target_path)).unwrap(),
            row.bytes
        );
    }
}

#[test]
pub(crate) fn exact_effect_failure_rolls_back_prior_bytes_and_settles_terminal() {
    let fixture = Fixture::new("rollback");
    fixture.write_template(CANONICAL_TEMPLATES[0].target_path);
    let context = fixture.context();
    let prepared = fixture.prepared(&context);
    let before = snapshot(&fixture.root);
    configure_effects_for_test(|effects| effects.fail_on_calls([2]));
    let outcome = execute(&fixture, &context, prepared, "rollback");
    assert_eq!(outcome.status(), "rolled_back");
    assert_eq!(outcome.error_id(), Some(AdapterErrorId::ApplyRolledBack));
    assert!(outcome.effect_started());
    assert!(outcome.rollback_complete());
    assert_eq!(snapshot(&fixture.root), before);
    assert!(
        String::from_utf8(fs::read(fixture.store.root.join("authority-ledger.json")).unwrap())
            .unwrap()
            .contains("rolled_back")
    );
}

#[test]
pub(crate) fn unexpired_contender_refuses_and_expired_pre_effect_reservation_reconciles_without_target_write()
 {
    let fixture = Fixture::new("interrupted");
    let context = fixture.context();
    let prepared = fixture.prepared(&context);
    let recovery_intent = prepare_recovery_intent(&context, &prepared).unwrap();
    let intent = recovery_intent.to_machine_bytes();
    let before = snapshot(&fixture.root);
    after_reservation_for_test(|| panic!("simulated process interruption"));
    let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        execute_prepared_apply(
            &context,
            prepared,
            recovery_intent,
            &TestClock::new([10, 11, 12]),
            &fixture.store,
            nonce("interrupted-first"),
        )
    }));
    assert!(interrupted.is_err());
    assert_eq!(snapshot(&fixture.root), before);

    let active_context = fixture.context();
    let active = recover_prepared_apply(
        &active_context,
        &intent,
        &TestClock::new([22]),
        &fixture.store,
        nonce("interrupted-first"),
    );
    assert_eq!(active.status(), "refused");
    assert_eq!(active.error_id(), Some(AdapterErrorId::ApplyLeaseInvalid));
    assert!(!active.effect_started());
    assert_eq!(snapshot(&fixture.root), before);

    let retry_context = fixture.context();
    let retry = recover_prepared_apply(
        &retry_context,
        &intent,
        &TestClock::new([73]),
        &fixture.store,
        nonce("interrupted-first"),
    );
    assert_eq!(retry.status(), "interrupted");
    assert_eq!(
        retry.error_id(),
        Some(AdapterErrorId::ApplyOutcomeAmbiguous)
    );
    assert!(!retry.effect_started());
    assert_eq!(snapshot(&fixture.root), before);
    assert!(
        String::from_utf8(fs::read(fixture.store.root.join("authority-ledger.json")).unwrap())
            .unwrap()
            .contains("interrupted")
    );
}
