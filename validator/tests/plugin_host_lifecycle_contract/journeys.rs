use crate::host_lifecycle::{HostLayer, HostLayerVerdict, HostLifecyclePhase};
use crate::plugin_product::lifecycle::{ApplyDisposition, LifecycleIntent, LifecycleState};
use crate::support::{Fixture, Reader, handoff_external_effect, installed, lifecycle, request};

#[test]
fn clean_install_keeps_all_host_layers_separate_and_requests_but_never_executes_host_effects() {
    let fixture = Fixture::new("clean-install");
    let bundle = fixture.bundle("0.0.12");
    let before = LifecycleState::default();
    let plan = lifecycle(
        &before,
        request(
            LifecycleIntent::FreshInstall,
            Some(bundle.authority.clone()),
            None,
            None,
            true,
            false,
        ),
    );
    let (mut session, host) = fixture.session(&bundle, plan);
    let applied = session.apply_confined(&before).unwrap();
    assert_eq!(applied.disposition, ApplyDisposition::Applied);
    let host_plan = handoff_external_effect(&mut session).unwrap();
    assert_eq!(host_plan.commands().len(), 1);
    let mut reader = Reader::complete(&bundle, &host, session.binding());
    let report = session.capture_and_verify(&mut reader).unwrap();
    assert_eq!(report.layers().len(), HostLayer::ALL.len());
    assert_eq!(
        report.layer(HostLayer::Package).verdict(),
        HostLayerVerdict::Verified
    );
    assert_eq!(
        report.layer(HostLayer::Marketplace).verdict(),
        HostLayerVerdict::Verified
    );
    assert_eq!(
        report.layer(HostLayer::Installed).verdict(),
        HostLayerVerdict::Verified
    );
    assert_eq!(
        report.layer(HostLayer::Cache).verdict(),
        HostLayerVerdict::Verified
    );
    assert_eq!(
        report.layer(HostLayer::AppRegistry).verdict(),
        HostLayerVerdict::Verified
    );
    assert_eq!(
        report.layer(HostLayer::Discovery).verdict(),
        HostLayerVerdict::Verified
    );
    assert_eq!(
        report.layer(HostLayer::Runtime).verdict(),
        HostLayerVerdict::Verified
    );
    assert_eq!(
        report.layer(HostLayer::PluginsUi).verdict(),
        HostLayerVerdict::Unsupported
    );
    assert_eq!(
        report.phase(),
        HostLifecyclePhase::AwaitingSupportedHostObservation
    );
    assert!(report.identity_chain_sha256().is_some());
    assert!(report.external_effect_request_sha256().is_some());
    assert!(report.external_effect_consumed());
    assert!(!report.has_claim_effect());
}

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
        fixture.confined(),
        &current.plan,
        &current.snapshot,
        repeat,
        host,
        crate::support::marketplace_plan(&current),
        crate::host_lifecycle::HostScopeAuthority::Personal {
            marketplace: "local-harness-plugins".into(),
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
