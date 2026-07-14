#[test]
fn dirty_repeat_use_is_recursive_zero_write() {
    let fixture = Fixture::new("dirty-repeat");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    std::fs::write(
        fixture.root.join("unrelated-user-work.txt"),
        b"preserve me\n",
    )
    .unwrap();
    let state = installed(&bundle.authority, 7);
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
    let before = fixture.tree();
    let (mut session, host) = fixture.session(&bundle, repeat);
    session.apply_confined(&state).unwrap();
    let mut reader = Reader::complete(&bundle, &host, session.binding());
    session.capture_and_verify(&mut reader).unwrap();
    assert_eq!(fixture.tree(), before);
}

#[test]
fn partial_install_repairs_only_cache_and_conflicting_physical_state_refuses_at_bind() {
    let fixture = Fixture::new("partial-conflict");
    let current = fixture.bundle("0.0.12");
    let wrong = fixture.bundle("0.0.13");
    fixture.seed(Some(&current), None);
    let partial = LifecycleState {
        installed: Some(current.authority.clone()),
        cache: None,
        generation: 4,
        recovery_required: false,
    };
    let refresh = lifecycle(
        &partial,
        request(
            LifecycleIntent::StaleCacheRecovery,
            None,
            None,
            partial.installed.as_ref(),
            true,
            false,
        ),
    );
    let (mut session, _) = fixture.session(&current, refresh);
    let report = session.apply_confined(&partial).unwrap();
    handoff_external_effect(&mut session).unwrap();
    assert_eq!(report.state.installed, report.state.cache);

    fixture.replace(
        "installed/harness-ultragoal.hugpkg",
        Some(&current.authority.package_sha256),
        Some(wrong.snapshot.archive()),
    );
    let repeat = lifecycle(
        &report.state,
        request(
            LifecycleIntent::RepeatUse,
            Some(current.authority.clone()),
            None,
            report.state.installed.as_ref(),
            false,
            false,
        ),
    );
    let before = fixture.tree();
    let host = fixture.host();
    let result = crate::host_lifecycle::HostLifecycleSession::bind(
        crate::host_lifecycle::HostLifecycleBindRequest {
            root: fixture.confined(),
            package_plan: &current.plan,
            package: &current.snapshot,
            lifecycle: repeat,
            host,
            marketplace_plan: crate::host_fixture::marketplace_plan(&current),
            host_scope: crate::host_lifecycle::HostScopeAuthority::Personal {
                marketplace: "local-harness-plugins".into(),
            },
        },
    );
    assert!(result.is_err());
    assert_eq!(fixture.tree(), before);
}

#[test]
fn scenario_fixture_is_exact_and_declares_no_claim_effect() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/plugin-host-lifecycle/scenarios.json");
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(
        value["schema"],
        "harness-ultragoal.plugin-host-lifecycle-scenarios.v1"
    );
    assert_eq!(value["claim_effect"], "none");
    let rows = value["scenarios"].as_array().unwrap();
    assert_eq!(rows.len(), 18);
    let ids = rows
        .iter()
        .map(|row| row["id"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids.len(), rows.len());
}
