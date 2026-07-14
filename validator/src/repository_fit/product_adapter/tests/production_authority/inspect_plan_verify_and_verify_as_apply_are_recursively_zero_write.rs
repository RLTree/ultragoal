use super::*;

#[test]
pub(crate) fn inspect_plan_verify_and_verify_as_apply_are_recursively_zero_write() {
    let fixture = Fixture::new("zero-write-reads");
    let context = fixture.context();
    let target_before = snapshot(&fixture.root);
    let status_before = git_status(&fixture.root);
    let _ = inspect_target(&context).unwrap();
    let _ = plan_target(&context).unwrap();
    let _verify = verify_target(&context).unwrap();
    assert_eq!(snapshot(&fixture.root), target_before);
    assert_eq!(git_status(&fixture.root), status_before);
    assert_eq!(fixture.store_names(), Vec::<String>::new());
}

#[test]
pub(crate) fn forged_desired_target_after_interruption_cannot_become_terminal_success() {
    let fixture = Fixture::new("forged-success");
    let context = fixture.context();
    let prepared = fixture.prepared(&context);
    let intent = prepare_recovery_intent(&context, &prepared).unwrap();
    let intent_bytes = intent.to_machine_bytes();
    after_reservation_for_test(|| panic!("interrupt before effect"));
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        execute_prepared_apply(
            &context,
            prepared,
            intent,
            &TestClock::new([10, 11, 12]),
            &fixture.store,
            nonce("forged-success-nonce"),
        )
    }));
    fixture.install_all_direct();
    let forged_context = fixture.context();
    let forged = recover_prepared_apply(
        &forged_context,
        &intent_bytes,
        &TestClock::new([73]),
        &fixture.store,
        nonce("forged-success-nonce"),
    );
    assert_eq!(forged.status(), "interrupted");
    assert_eq!(
        forged.error_id(),
        Some(AdapterErrorId::ApplyOutcomeAmbiguous)
    );
    let ledger =
        String::from_utf8(fs::read(fixture.store.root.join("authority-ledger.json")).unwrap())
            .unwrap();
    assert!(!ledger.contains("committed"));
}

#[test]
pub(crate) fn ledger_rejects_key_lock_state_root_and_special_file_substitution() {
    for attack in [
        "key",
        "lock",
        "state",
        "truncated",
        "unknown-row",
        "hardlink",
        "root-mode",
    ] {
        let fixture = Fixture::new(&format!("ledger-{attack}"));
        let ledger = FileRepositoryFitLedger::open_or_initialize(
            &fixture.store.root,
            fixture.store.store_id(),
        )
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
        let key = fixture.store.root.join("authority.key");
        let lock = fixture.store.root.join("authority.lock");
        let state = fixture.store.root.join("authority-ledger.json");
        match attack {
            "key" => {
                let held = fixture.store.root.join("authority.key.held");
                fs::rename(&key, held).unwrap();
                fs::write(&key, [7u8; 32]).unwrap();
                fs::set_permissions(&key, fs::Permissions::from_mode(0o600)).unwrap();
            }
            "lock" => {
                let held = fixture.store.root.join("authority.lock.held");
                fs::rename(&lock, held).unwrap();
                fs::write(&lock, b"repository-fit-authority-lock-v2\n").unwrap();
                fs::set_permissions(&lock, fs::Permissions::from_mode(0o600)).unwrap();
            }
            "state" => fs::write(&state, b"{}").unwrap(),
            "truncated" => {
                let bytes = fs::read(&state).unwrap();
                fs::write(&state, &bytes[..bytes.len() / 2]).unwrap();
            }
            "unknown-row" => {
                let bytes = String::from_utf8(fs::read(&state).unwrap()).unwrap();
                fs::write(&state, bytes.replacen('{', "{\"unknown\":true,", 1)).unwrap();
            }
            "hardlink" => {
                fs::hard_link(
                    &state,
                    fixture.store.root.join("authority-ledger-copy.json"),
                )
                .unwrap();
            }
            "root-mode" => {
                fs::set_permissions(&fixture.store.root, fs::Permissions::from_mode(0o755))
                    .unwrap();
            }
            _ => unreachable!(),
        }
        let result = ledger.reserve(reservation('a'));
        assert!(
            matches!(result, Err(ref error) if matches!(error.id(), LedgerErrorId::Tampered | LedgerErrorId::InvalidStore | LedgerErrorId::Io)),
            "attack {attack} did not fail closed"
        );
    }

    for kind in ["symlink-root", "fifo-key", "unknown-without-lock"] {
        let fixture = Fixture::new(kind);
        let requested = if kind == "symlink-root" {
            let real = fixture.container.join("real-authority");
            fs::create_dir(&real).unwrap();
            fs::set_permissions(&real, fs::Permissions::from_mode(0o700)).unwrap();
            let alias = fixture.container.join("authority-alias");
            symlink(&real, &alias).unwrap();
            alias
        } else if kind == "fifo-key" {
            let key = fixture.store.root.join("authority.key");
            let path = std::ffi::CString::new(key.as_os_str().as_encoded_bytes()).unwrap();
            assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
            fixture.store.root.clone()
        } else {
            fs::write(fixture.store.root.join("unrecognized-row"), b"attacker\n").unwrap();
            fixture.store.root.clone()
        };
        let before = fs::read_dir(&requested)
            .map(|rows| rows.count())
            .unwrap_or_default();
        let result =
            FileRepositoryFitLedger::open_or_initialize(&requested, fixture.store.store_id());
        assert!(result.is_err());
        let after = fs::read_dir(&requested)
            .map(|rows| rows.count())
            .unwrap_or_default();
        assert_eq!(after, before);
        assert!(!requested.join("authority.lock").exists());
    }
}
