#[test]
fn update_recovery_rollback_reinstall_stale_cache_repeat_and_uninstall_share_one_authority() {
    let fixture = Fixture::new("all-intents");
    let v11 = fixture.bundle("0.0.11");
    let v12 = fixture.bundle("0.0.12");
    let empty = LifecycleState::default();

    let fresh = lifecycle(
        &empty,
        request(
            LifecycleIntent::FreshInstall,
            Some(v11.authority.clone()),
            None,
            None,
            true,
            false,
        ),
    );
    let (mut session, host) = fixture.session(&v11, fresh);
    let mut state = session.apply_confined(&empty).unwrap().state;
    handoff_external_effect(&mut session).unwrap();
    let mut reader = Reader::complete(&v11, &host, session.binding());
    session.capture_and_verify(&mut reader).unwrap();

    let update = lifecycle(
        &state,
        request(
            LifecycleIntent::MonotonicUpdate,
            Some(v12.authority.clone()),
            None,
            state.installed.as_ref(),
            true,
            false,
        ),
    );
    let (mut session, host) = fixture.session(&v12, update);
    state = session.apply_confined(&state).unwrap().state;
    handoff_external_effect(&mut session).unwrap();
    let mut reader = Reader::complete(&v12, &host, session.binding());
    session.capture_and_verify(&mut reader).unwrap();

    let rollback = lifecycle(
        &state,
        request(
            LifecycleIntent::AuthorizedRollback,
            Some(v11.authority.clone()),
            None,
            state.installed.as_ref(),
            true,
            true,
        ),
    );
    let (mut session, host) = fixture.session(&v11, rollback);
    state = session.apply_confined(&state).unwrap().state;
    handoff_external_effect(&mut session).unwrap();
    let mut reader = Reader::complete(&v11, &host, session.binding());
    session.capture_and_verify(&mut reader).unwrap();

    for intent in [
        LifecycleIntent::IdempotentReinstall,
        LifecycleIntent::RepeatUse,
    ] {
        let read_only = lifecycle(
            &state,
            request(
                intent,
                Some(v11.authority.clone()),
                None,
                state.installed.as_ref(),
                false,
                false,
            ),
        );
        let before = fixture.tree();
        let (mut session, host) = fixture.session(&v11, read_only);
        let report = session.apply_confined(&state).unwrap();
        let mut reader = Reader::complete(&v11, &host, session.binding());
        let verified = session.capture_and_verify(&mut reader).unwrap();
        assert_eq!(report.state, state);
        assert!(verified.external_effect_request_sha256().is_none());
        assert!(!verified.external_effect_consumed());
        assert_eq!(
            fixture.tree(),
            before,
            "read-only lifecycle wrote to fixture"
        );
    }

    fixture.replace(
        "cache/harness-ultragoal.hugpkg",
        Some(&v11.authority.package_sha256),
        Some(v12.snapshot.archive()),
    );
    let stale = LifecycleState {
        cache: Some(v12.authority.clone()),
        ..state.clone()
    };
    let refresh = lifecycle(
        &stale,
        request(
            LifecycleIntent::StaleCacheRecovery,
            None,
            None,
            stale.installed.as_ref(),
            true,
            false,
        ),
    );
    let installed_before = fixture
        .tree()
        .into_iter()
        .find(|row| row.0 == "installed/harness-ultragoal.hugpkg")
        .unwrap();
    let (mut session, host) = fixture.session(&v11, refresh);
    state = session.apply_confined(&stale).unwrap().state;
    handoff_external_effect(&mut session).unwrap();
    let mut reader = Reader::complete(&v11, &host, session.binding());
    session.capture_and_verify(&mut reader).unwrap();
    let installed_after = fixture
        .tree()
        .into_iter()
        .find(|row| row.0 == "installed/harness-ultragoal.hugpkg")
        .unwrap();
    assert_eq!(
        installed_before, installed_after,
        "cache repair rewrote install"
    );

    fixture.replace(
        "installed/harness-ultragoal.hugpkg",
        Some(&v11.authority.package_sha256),
        Some(v12.snapshot.archive()),
    );
    let interrupted = LifecycleState {
        installed: Some(v12.authority.clone()),
        cache: Some(v11.authority.clone()),
        generation: state.generation + 1,
        recovery_required: true,
    };
    let recovery = lifecycle(
        &interrupted,
        request(
            LifecycleIntent::FailedUpdateRecovery,
            None,
            Some(state.clone()),
            interrupted.installed.as_ref(),
            true,
            true,
        ),
    );
    let (mut session, host) = fixture.session(&v11, recovery);
    state = session.apply_confined(&interrupted).unwrap().state;
    handoff_external_effect(&mut session).unwrap();
    let mut reader = Reader::complete(&v11, &host, session.binding());
    session.capture_and_verify(&mut reader).unwrap();

    let teardown = lifecycle(
        &state,
        request(
            LifecycleIntent::UninstallTeardown,
            None,
            None,
            state.installed.as_ref(),
            true,
            false,
        ),
    );
    let (mut session, _) = fixture.session(&v11, teardown);
    session.apply_confined(&state).unwrap();
    handoff_external_effect(&mut session).unwrap();
    let mut reader = Reader::teardown(&v11, session.binding());
    let report = session.capture_and_verify(&mut reader).unwrap();
    assert_eq!(report.phase(), HostLifecyclePhase::TeardownObserved);
    assert!(report.state().installed.is_none() && report.state().cache.is_none());
    assert!(report.external_effect_request_sha256().is_some());
    assert!(report.external_effect_consumed());
}
