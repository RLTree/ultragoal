#[test]
fn ledger_and_key_identity_substitution_fail_closed() {
    let fixture = LedgerFixture::new();
    let ledger = FileHostEffectLedger::create(&fixture.root, "host-ledger".to_owned()).unwrap();
    let wrong = FileHostEffectLedger::open(&fixture.root, "other-ledger".to_owned());
    assert!(matches!(
        wrong,
        Err(error) if error.id() == HostEffectLedgerErrorId::Tampered
    ));

    let key = fixture.root.join(KEY_NAME);
    fs::rename(&key, fixture.root.join("held.key")).unwrap();
    let mut replacement = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&key)
        .unwrap();
    replacement.write_all(&[7_u8; KEY_BYTES]).unwrap();
    replacement.sync_all().unwrap();
    assert_eq!(
        ledger.head().unwrap_err().id(),
        HostEffectLedgerErrorId::Tampered
    );
}

#[test]
fn key_replacement_between_identity_and_read_cannot_splice_state() {
    let fixture = LedgerFixture::new();
    let ledger =
        Arc::new(FileHostEffectLedger::create(&fixture.root, "host-ledger".to_owned()).unwrap());
    let reached = Arc::new(std::sync::Barrier::new(2));
    let release = Arc::new(std::sync::Barrier::new(2));
    ledger
        .install_key_open_hook(KeyOpenHook {
            reached: Arc::clone(&reached),
            release: Arc::clone(&release),
        })
        .unwrap();

    let contender = Arc::clone(&ledger);
    let worker = thread::spawn(move || contender.head());
    reached.wait();

    let named_key = fixture.root.join(KEY_NAME);
    let held_key = fixture.root.with_extension("held-key");
    fs::rename(&named_key, &held_key).unwrap();
    let replacement_bytes = [7_u8; KEY_BYTES];
    let mut replacement = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&named_key)
        .unwrap();
    replacement.write_all(&replacement_bytes).unwrap();
    replacement.sync_all().unwrap();

    let replacement_key = LedgerKey(replacement_bytes);
    let payload = initial_payload(
        "host-ledger",
        &digest(&replacement_key.0),
        ledger.lock_identity,
    )
    .unwrap();
    let replacement_state = encode_snapshot(&payload, &replacement_key).unwrap();
    overwrite(&fixture.root.join(STATE_NAME), &replacement_state);
    release.wait();

    assert_eq!(
        worker.join().unwrap().unwrap_err().id(),
        HostEffectLedgerErrorId::Tampered
    );
    assert_eq!(fs::read(named_key).unwrap(), replacement_bytes);
    assert_eq!(
        fs::read(fixture.root.join(STATE_NAME)).unwrap(),
        replacement_state
    );
}

#[test]
fn lock_replacement_after_prelock_check_cannot_split_serialization() {
    let fixture = LedgerFixture::new();
    let ledger =
        Arc::new(FileHostEffectLedger::create(&fixture.root, "host-ledger".to_owned()).unwrap());
    let state_before = fs::read(fixture.root.join(STATE_NAME)).unwrap();
    let reached = Arc::new(std::sync::Barrier::new(2));
    let release = Arc::new(std::sync::Barrier::new(2));
    ledger
        .install_lock_open_hook(LockOpenHook {
            reached: Arc::clone(&reached),
            release: Arc::clone(&release),
        })
        .unwrap();

    let contender = Arc::clone(&ledger);
    let worker = thread::spawn(move || contender.head());
    reached.wait();

    let named_lock = fixture.root.join(LOCK_NAME);
    let held_lock = fixture.root.with_extension("held-lock");
    fs::rename(&named_lock, &held_lock).unwrap();
    let replacement = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&named_lock)
        .unwrap();
    replacement.sync_all().unwrap();
    release.wait();

    assert_eq!(
        worker.join().unwrap().unwrap_err().id(),
        HostEffectLedgerErrorId::Tampered
    );
    assert_eq!(
        fs::read(fixture.root.join(STATE_NAME)).unwrap(),
        state_before
    );
    let reopened = FileHostEffectLedger::open(&fixture.root, "host-ledger".to_owned());
    assert!(matches!(
        reopened,
        Err(error) if error.id() == HostEffectLedgerErrorId::Tampered
    ));
    assert_eq!(
        fs::read(fixture.root.join(STATE_NAME)).unwrap(),
        state_before
    );
}

#[test]
fn unknown_and_special_ledger_entries_fail_closed_without_cleanup() {
    let fixture = LedgerFixture::new();
    let ledger = FileHostEffectLedger::create(&fixture.root, "host-ledger".to_owned()).unwrap();
    let unknown = fixture.root.join("unknown.json");
    fs::write(&unknown, b"{}\n").unwrap();
    fs::set_permissions(&unknown, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        ledger.head().unwrap_err().id(),
        HostEffectLedgerErrorId::Tampered
    );
    assert!(unknown.exists());
    fs::remove_file(&unknown).unwrap();

    let state = fixture.root.join(STATE_NAME);
    let held = fixture.root.join("held-state");
    fs::rename(&state, &held).unwrap();
    symlink(&held, &state).unwrap();
    assert_eq!(
        ledger.head().unwrap_err().id(),
        HostEffectLedgerErrorId::Tampered
    );
    assert!(
        fs::symlink_metadata(&state)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}
