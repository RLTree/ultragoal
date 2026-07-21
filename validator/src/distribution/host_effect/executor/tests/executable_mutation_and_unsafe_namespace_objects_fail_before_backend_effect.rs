#[test]
fn executable_mutation_and_unsafe_namespace_objects_fail_before_backend_effect() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let ledger =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &ledger, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut changed = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&fixture.executable)
        .unwrap();
    changed.write_all(b"#!/bin/sh\nexit 17\n").unwrap();
    changed.sync_all().unwrap();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let mut backend = ScriptedBackend::success();
    let mut executor = SupportedHostEffectExecutor::new(
        &ledger,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );
    let failure = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(failure.id(), HostEffectExecutorErrorId::ExecutableMutation);
    assert_eq!(failure.terminal_state(), Some(HostEffectState::Failed));
    drop(executor);
    assert_eq!(backend.calls, 0);
    assert_eq!(
        ledger.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::Failed
    );

    for object in ["symlink", "hardlink", "fifo", "temp-collision"] {
        let fixture = Fixture::new();
        let (_scope, target, _target_identity) = fixture.scope_and_target();
        let target_name = format!("effect-{}.json", "a".repeat(64));
        let path = fixture.target_root.join(&target_name);
        match object {
            "symlink" => std::os::unix::fs::symlink("/private/tmp", &path).unwrap(),
            "hardlink" => {
                let source = fixture.target_root.join("unrelated-source");
                let mut file = OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&source)
                    .unwrap();
                file.write_all(b"linked").unwrap();
                file.sync_all().unwrap();
                fs::hard_link(source, &path).unwrap();
            }
            "fifo" => {
                let path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
            }
            "temp-collision" => {
                let temp = fixture
                    .target_root
                    .join(format!(".{target_name}.0000000000000000.tmp"));
                let file = OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(temp)
                    .unwrap();
                file.sync_all().unwrap();
            }
            _ => unreachable!(),
        }
        let failure = target
            .require_clean_publication_name(&target_name)
            .unwrap_err();
        let expected = if object == "temp-collision" {
            HostEffectExecutorErrorId::TempCollision
        } else {
            HostEffectExecutorErrorId::UnsafeObject
        };
        assert_eq!(failure.id(), expected, "object={object}");
    }
}

#[test]
fn started_timeout_is_ambiguous_and_never_publishes() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let ledger =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &ledger, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let mut backend = ScriptedBackend {
        replies: VecDeque::from([BackendReply::Failure(
            HostEffectExecutorErrorId::Timeout,
            true,
        )]),
        calls: 0,
    };
    let mut executor = SupportedHostEffectExecutor::new(
        &ledger,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );
    let failure = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(failure.id(), HostEffectExecutorErrorId::Timeout);
    assert_eq!(failure.terminal_state(), Some(HostEffectState::Ambiguous));
    assert_eq!(
        ledger.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::Ambiguous
    );
    assert!(fs::read_dir(&fixture.target_root).unwrap().next().is_none());
}

#[cfg(target_os = "macos")]
#[test]
fn native_darwin_backend_refuses_before_spawn_or_output() {
    let fixture = Fixture::new();
    let executable = SelectedCodexExecutable::pin_for_test_fixture(&fixture.executable).unwrap();
    let plan =
        HostCommandPlan::personal_install(&fixture.package(), "fixture-marketplace").unwrap();
    let mut backend = NativeRetainedDescriptorProcessBackend;
    let failure = backend
        .execute(
            &capability(),
            &executable,
            &plan.commands()[0],
            &HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
            &HostEffectCancellation::default(),
            0,
        )
        .unwrap_err();
    assert_eq!(failure.id, HostEffectExecutorErrorId::UnsupportedPlatform);
    assert!(!failure.started);
    assert!(failure.capture.stdout.is_empty());
    assert!(failure.capture.stderr.is_empty());
}
