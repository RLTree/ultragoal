#[test]
fn rename_race_and_output_overflow_never_settle() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let ledger =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &ledger, &target_identity);
    let target_name = receipt_name(effect.permit().permit_id()).unwrap();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    set_fault(FaultPoint::BeforeRename, false, move |root| {
        let path = root.join(&target_name);
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .unwrap();
        file.write_all(b"racer").unwrap();
        file.sync_all().unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o400)).unwrap();
    });
    let mut backend = ScriptedBackend::success();
    let policy = HostEffectExecutionPolicy::strict(10_000, &[]).unwrap();
    let mut executor = SupportedHostEffectExecutor::new(&ledger, target, &mut backend, policy);
    let failure = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(failure.id(), HostEffectExecutorErrorId::RenameRace);
    assert_eq!(failure.terminal_state(), Some(HostEffectState::Ambiguous));

    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let ledger =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &ledger, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let mut backend = ScriptedBackend {
        replies: VecDeque::from([BackendReply::Oversized]),
        calls: 0,
    };
    let policy = HostEffectExecutionPolicy::strict(10_000, &[]).unwrap();
    let mut executor = SupportedHostEffectExecutor::new(&ledger, target, &mut backend, policy);
    let failure = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(failure.id(), HostEffectExecutorErrorId::OutputOverflow);
    assert_eq!(
        ledger.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::Ambiguous
    );
}

#[test]
fn environment_injection_and_prestart_cancel_fail_without_backend_effect() {
    assert_eq!(
        HostEffectExecutionPolicy::strict(
            10_000,
            &[("DYLD_INSERT_LIBRARIES".to_owned(), "/tmp/inject".to_owned())]
        )
        .unwrap_err()
        .id(),
        HostEffectExecutorErrorId::EnvironmentInjection
    );

    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let ledger =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &ledger, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let mut backend = ScriptedBackend::success();
    let cancellation = HostEffectCancellation::default();
    cancellation.cancel();
    let policy = HostEffectExecutionPolicy::strict(10_000, &[]).unwrap();
    let mut executor = SupportedHostEffectExecutor::new(&ledger, target, &mut backend, policy);
    let failure = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &cancellation,
        )
        .unwrap_err();
    assert_eq!(failure.id(), HostEffectExecutorErrorId::Cancelled);
    assert_eq!(failure.terminal_state(), Some(HostEffectState::Failed));
    assert_eq!(backend.calls, 0);
    assert_eq!(
        ledger.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::Failed
    );
    assert!(fs::read_dir(&fixture.target_root).unwrap().next().is_none());
}
