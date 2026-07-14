const EFFECTFUL: [LifecycleIntent; 6] = [
    LifecycleIntent::FreshInstall,
    LifecycleIntent::MonotonicUpdate,
    LifecycleIntent::FailedUpdateRecovery,
    LifecycleIntent::AuthorizedRollback,
    LifecycleIntent::UninstallTeardown,
    LifecycleIntent::StaleCacheRecovery,
];

#[test]
fn absent_host_capabilities_block_every_effectful_intent_before_any_write_or_plan() {
    for intent in EFFECTFUL {
        let (fixture, mut session, observed, canary) = unavailable_session(intent);
        let before = fixture.tree();
        let error = session.apply_confined(&observed).unwrap_err();
        assert_eq!(error.id(), HostLifecycleErrorId::HostCapabilityRejected);
        assert!(!error.to_string().contains(&canary));
        assert_eq!(fixture.tree(), before, "{intent:?} wrote before gate");
        assert_eq!(
            session.apply_confined(&observed).unwrap_err().id(),
            HostLifecycleErrorId::SessionStateRejected
        );
        assert_eq!(
            session.take_external_effect_request().unwrap_err().id(),
            HostLifecycleErrorId::HostCapabilityRejected
        );
        assert_eq!(fixture.tree(), before, "{intent:?} revived after rejection");
    }
}

#[test]
fn unsupported_and_absent_states_fail_the_exact_intent_and_scope_capability_sets() {
    for intent in EFFECTFUL {
        let personal = HostScopeAuthority::Personal {
            marketplace: "local-harness-plugins".into(),
        };
        let required = required_host_capabilities(intent, &personal);
        assert_eq!(required, [Capability::Install, Capability::Marketplace]);
        assert!(!capability_states_supported(required, |_| {
            HostCapabilityState::Absent
        }));
        assert!(!capability_states_supported(required, |_| {
            HostCapabilityState::Unsupported
        }));
        assert!(capability_states_supported(required, |_| {
            HostCapabilityState::Supported
        }));

        let repository = HostScopeAuthority::Repository {
            repository_root: "/tmp/repository".into(),
            marketplace: "local-harness-plugins".into(),
        };
        assert_eq!(
            required_host_capabilities(intent, &repository),
            [
                Capability::Filesystem,
                Capability::Install,
                Capability::Marketplace
            ]
        );
    }
}

#[test]
fn one_scope_authority_eliminates_both_marketplace_effect_mismatch_directions() {
    for intent in EFFECTFUL {
        let personal = HostScopeAuthority::Personal {
            marketplace: "local-harness-plugins".into(),
        };
        assert!(personal.accepts_marketplace_scope(MarketplaceScope::Personal));
        assert!(!personal.accepts_marketplace_scope(MarketplaceScope::Repository));
        assert_eq!(
            required_host_capabilities(intent, &personal),
            [Capability::Install, Capability::Marketplace]
        );

        let repository = HostScopeAuthority::Repository {
            repository_root: "/tmp/repository".into(),
            marketplace: "local-harness-plugins".into(),
        };
        assert!(repository.accepts_marketplace_scope(MarketplaceScope::Repository));
        assert!(!repository.accepts_marketplace_scope(MarketplaceScope::Personal));
        assert_eq!(
            required_host_capabilities(intent, &repository),
            [
                Capability::Filesystem,
                Capability::Install,
                Capability::Marketplace
            ]
        );
    }

    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/plugin_product/host_lifecycle/session.rs"),
    )
    .unwrap();
    assert!(!source.contains("marketplace_scope: MarketplaceScope"));
    assert!(!source.contains("effect_scope:"));
}

#[test]
fn marketplace_name_substitution_refuses_before_operation_or_issuance() {
    let fixture = Fixture::new("marketplace-name-scope-substitution");
    let bundle = fixture.bundle("0.0.12");
    let empty = LifecycleState::default();
    for (plan_name, scope_name) in [
        ("other-marketplace", "local-harness-plugins"),
        ("local-harness-plugins", "other-marketplace"),
    ] {
        let before = fixture.tree();
        let result = crate::host_lifecycle::HostLifecycleSession::bind(
            crate::host_lifecycle::HostLifecycleBindRequest {
                root: fixture.confined(),
                package_plan: &bundle.plan,
                package: &bundle.snapshot,
                lifecycle: fresh_plan(&bundle, &empty),
                host: fixture.host(),
                marketplace_plan: marketplace_plan_named(&bundle, plan_name),
                host_scope: HostScopeAuthority::Personal {
                    marketplace: scope_name.into(),
                },
            },
        );
        assert_eq!(
            result.err().unwrap().id(),
            HostLifecycleErrorId::InvalidBinding
        );
        assert_eq!(fixture.tree(), before);
    }
}

#[test]
fn personal_and_repository_requests_cannot_cross_scope_sessions() {
    let fixture = Fixture::new("cross-host-scope-request");
    let bundle = fixture.bundle("0.0.12");
    let empty = LifecycleState::default();
    let mut personal = fixture.session_with_scope(
        &bundle,
        fresh_plan(&bundle, &empty),
        fixture.host(),
        HostScopeAuthority::Personal {
            marketplace: "local-harness-plugins".into(),
        },
    );
    let mut repository = fixture.session_with_scope(
        &bundle,
        fresh_plan(&bundle, &empty),
        fixture.host(),
        HostScopeAuthority::Repository {
            repository_root: fixture.project.to_string_lossy().into_owned(),
            marketplace: "local-harness-plugins".into(),
        },
    );
    personal.apply_confined(&empty).unwrap();
    let personal_request = personal.take_external_effect_request().unwrap();
    fixture.replace(
        "installed/harness-ultragoal.hugpkg",
        Some(&bundle.authority.package_sha256),
        None,
    );
    fixture.replace(
        "cache/harness-ultragoal.hugpkg",
        Some(&bundle.authority.package_sha256),
        None,
    );
    repository.apply_confined(&empty).unwrap();
    assert_ne!(personal.host_scope_sha256(), repository.host_scope_sha256());
    let before = fixture.tree();
    assert_eq!(
        repository
            .consume_external_effect_request(personal_request)
            .unwrap_err()
            .id(),
        HostLifecycleErrorId::ExternalEffectSessionMismatch
    );
    assert_eq!(fixture.tree(), before);
}
