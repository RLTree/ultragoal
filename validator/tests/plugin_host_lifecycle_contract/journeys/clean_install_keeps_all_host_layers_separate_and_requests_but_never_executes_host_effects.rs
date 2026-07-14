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
