#[test]
fn stale_scope_rejected_session_wrong_state_and_effect_replay_never_reach_adapter() {
    {
        let fixture = Fixture::new("preflight-stale-scope-rejected");
        let bundle = fixture.bundle("0.0.12");
        fixture.seed(Some(&bundle), Some(&bundle));
        let state = installed(&bundle.authority, 20);
        let repeat = lifecycle(
            &state,
            request(
                LifecycleIntent::RepeatUse,
                Some(bundle.authority.clone()),
                None,
                state.installed.as_ref(),
                false,
                false,
            ),
        );
        let host = fixture.host();
        let mut session = fixture.session_with_scope(
            &bundle,
            repeat,
            host,
            HostScopeAuthority::Repository {
                repository_root: fixture.project.to_string_lossy().into_owned(),
                marketplace: "local-harness-plugins".into(),
            },
        );
        session.apply_confined(&state).unwrap();
        let original = fixture.root.join("preflight-original-project");
        std::fs::rename(&fixture.project, &original).unwrap();
        std::fs::create_dir(&fixture.project).unwrap();
        let before = fixture.tree();
        let mut stale = CountingUnsupportedReader::default();
        let error = session.capture_and_verify(&mut stale).unwrap_err();
        assert_eq!(error.id(), HostLifecycleErrorId::HostScopeRejected);
        assert!(stale.counts.is_zero());
        assert_eq!(fixture.tree(), before);

        std::fs::remove_dir(&fixture.project).unwrap();
        std::fs::rename(&original, &fixture.project).unwrap();
        let restored = fixture.tree();
        let mut rejected = CountingUnsupportedReader::default();
        let error = session.capture_and_verify(&mut rejected).unwrap_err();
        assert_eq!(error.id(), HostLifecycleErrorId::SessionStateRejected);
        assert!(rejected.counts.is_zero());
        assert_eq!(fixture.tree(), restored);
    }

    {
        let fixture = Fixture::new("preflight-wrong-confined-state");
        let bundle = fixture.bundle("0.0.12");
        fixture.seed(Some(&bundle), Some(&bundle));
        let state = installed(&bundle.authority, 21);
        let repeat = lifecycle(
            &state,
            request(
                LifecycleIntent::RepeatUse,
                Some(bundle.authority.clone()),
                None,
                state.installed.as_ref(),
                false,
                false,
            ),
        );
        let (mut session, _) = fixture.session(&bundle, repeat);
        session.apply_confined(&state).unwrap();
        fixture.replace(
            "installed/harness-ultragoal.hugpkg",
            Some(&bundle.authority.package_sha256),
            None,
        );
        let before = fixture.tree();
        let mut reader = CountingUnsupportedReader::default();
        let error = session.capture_and_verify(&mut reader).unwrap_err();
        assert_eq!(error.id(), HostLifecycleErrorId::IdentityMismatch);
        assert!(reader.counts.is_zero());
        assert_eq!(fixture.tree(), before);
    }

    {
        let fixture = Fixture::new("preflight-effect-replay");
        let bundle = fixture.bundle("0.0.12");
        let empty = LifecycleState::default();
        let fresh = lifecycle(
            &empty,
            request(
                LifecycleIntent::FreshInstall,
                Some(bundle.authority.clone()),
                None,
                None,
                true,
                false,
            ),
        );
        let (mut session, host) = fixture.session(&bundle, fresh);
        session.apply_confined(&empty).unwrap();
        let effect_request = session.take_external_effect_request().unwrap();
        session
            .consume_external_effect_request(effect_request)
            .unwrap()
            .into_plan()
            .unwrap();
        let mut current = Reader::complete(&bundle, &host, session.binding());
        session.capture_and_verify(&mut current).unwrap();
        let before = fixture.tree();
        let mut replay = CountingUnsupportedReader::default();
        let error = session.capture_and_verify(&mut replay).unwrap_err();
        assert_eq!(error.id(), HostLifecycleErrorId::SessionStateRejected);
        assert!(replay.counts.is_zero());
        assert_eq!(fixture.tree(), before);
    }
}

#[test]
fn mutation_after_preflight_is_rechecked_inside_transaction_before_surface_reads() {
    let fixture = Fixture::new("preflight-to-transaction-confined-state-race");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 22);
    let repeat = lifecycle(
        &state,
        request(
            LifecycleIntent::RepeatUse,
            Some(bundle.authority.clone()),
            None,
            state.installed.as_ref(),
            false,
            false,
        ),
    );
    let (mut session, host) = fixture.session(&bundle, repeat);
    session.apply_confined(&state).unwrap();
    let installed_path = fixture.root.join("installed/harness-ultragoal.hugpkg");
    let mut reader = CountingMutationReader {
        inner: Reader::complete(&bundle, &host, session.binding()),
        counts: AdapterInvocationCounts::default(),
        before_transaction: Some(Box::new(move || {
            std::fs::remove_file(installed_path).unwrap();
        })),
    };
    let error = session.capture_and_verify(&mut reader).unwrap_err();
    assert_eq!(error.id(), HostLifecycleErrorId::IdentityMismatch);
    assert_eq!(reader.counts.transactions.get(), 1);
    assert!(reader.counts.start_generation.get() > 0);
    assert!(reader.counts.current_generation.get() > 0);
    assert_eq!(reader.counts.read_count(), 0);
}
