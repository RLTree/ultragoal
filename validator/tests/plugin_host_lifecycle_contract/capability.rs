use crate::distribution::{
    Capability, HostCapabilityDeclaration, HostCapabilityState, MarketplaceScope,
};
use crate::host_lifecycle::{
    HostLifecycleErrorId, HostScopeAuthority, capability_states_supported,
    required_host_capabilities,
};
use crate::plugin_product::lifecycle::{LifecycleIntent, LifecycleState};
use crate::support::{Fixture, installed, lifecycle, marketplace_plan_named, request};

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
            fixture.confined(),
            &bundle.plan,
            &bundle.snapshot,
            fresh_plan(&bundle, &empty),
            fixture.host(),
            marketplace_plan_named(&bundle, plan_name),
            HostScopeAuthority::Personal {
                marketplace: scope_name.into(),
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

#[test]
fn no_effect_intents_do_not_promote_or_require_external_host_capability() {
    for intent in [
        LifecycleIntent::RepeatUse,
        LifecycleIntent::IdempotentReinstall,
    ] {
        let fixture = Fixture::new(&format!("no-effect-{intent:?}"));
        let bundle = fixture.bundle("0.0.12");
        fixture.seed(Some(&bundle), Some(&bundle));
        let observed = installed(&bundle.authority, 4);
        let plan = lifecycle(
            &observed,
            request(
                intent,
                Some(bundle.authority.clone()),
                None,
                observed.installed.as_ref(),
                false,
                false,
            ),
        );
        let host = HostCapabilityDeclaration::unavailable_codex_app(
            &fixture.root,
            &fixture.project,
            "unavailable-no-effect",
        )
        .unwrap();
        let mut session = fixture.session_with_scope(
            &bundle,
            plan,
            host,
            HostScopeAuthority::Personal {
                marketplace: "local-harness-plugins".into(),
            },
        );
        let before = fixture.tree();
        session.apply_confined(&observed).unwrap();
        assert_eq!(
            session.take_external_effect_request().unwrap_err().id(),
            HostLifecycleErrorId::ExternalEffectNotEligible
        );
        assert_eq!(fixture.tree(), before);
    }
}

#[test]
fn repository_scope_mismatch_and_each_handoff_race_fail_closed() {
    let mismatch = Fixture::new("repository-scope-mismatch");
    let bundle = mismatch.bundle("0.0.12");
    let other = mismatch.root.join("other-project");
    std::fs::create_dir_all(&other).unwrap();
    let empty = LifecycleState::default();
    let plan = fresh_plan(&bundle, &empty);
    let before = mismatch.tree();
    let result = crate::host_lifecycle::HostLifecycleSession::bind(
        mismatch.confined(),
        &bundle.plan,
        &bundle.snapshot,
        plan,
        mismatch.host(),
        crate::support::marketplace_plan(&bundle),
        HostScopeAuthority::Repository {
            repository_root: other.to_string_lossy().into_owned(),
            marketplace: "local-harness-plugins".into(),
        },
    );
    assert_eq!(
        result.err().unwrap().id(),
        HostLifecycleErrorId::HostScopeRejected
    );
    assert_eq!(mismatch.tree(), before);

    for boundary in ["take", "consume", "release"] {
        let fixture = Fixture::new(&format!("repository-scope-race-{boundary}"));
        let bundle = fixture.bundle("0.0.12");
        let empty = LifecycleState::default();
        let mut session = fixture.session_with_scope(
            &bundle,
            fresh_plan(&bundle, &empty),
            fixture.host(),
            HostScopeAuthority::Repository {
                repository_root: fixture.project.to_string_lossy().into_owned(),
                marketplace: "local-harness-plugins".into(),
            },
        );
        session.apply_confined(&empty).unwrap();
        let request = if boundary == "consume" {
            Some(session.take_external_effect_request().unwrap())
        } else {
            None
        };
        let prepared = if boundary == "release" {
            let request = session.take_external_effect_request().unwrap();
            Some(session.consume_external_effect_request(request).unwrap())
        } else {
            None
        };
        let original = fixture.root.join(format!("project-original-{boundary}"));
        std::fs::rename(&fixture.project, &original).unwrap();
        std::fs::create_dir(&fixture.project).unwrap();
        let after_mutation = fixture.tree();
        let error = match boundary {
            "take" => session.take_external_effect_request().unwrap_err(),
            "consume" => session
                .consume_external_effect_request(request.unwrap())
                .unwrap_err(),
            "release" => prepared.unwrap().into_plan().unwrap_err(),
            _ => unreachable!(),
        };
        assert_eq!(error.id(), HostLifecycleErrorId::HostScopeRejected);
        assert!(!error.to_string().contains(&original.to_string_lossy()[..]));
        assert_eq!(fixture.tree(), after_mutation, "{boundary} gate wrote");

        std::fs::remove_dir(&fixture.project).unwrap();
        std::fs::rename(&original, &fixture.project).unwrap();
        let restored = fixture.tree();
        assert_eq!(
            session.take_external_effect_request().unwrap_err().id(),
            HostLifecycleErrorId::ExternalEffectNotEligible
        );
        assert_eq!(fixture.tree(), restored, "{boundary} rejection revived");
    }
}

#[test]
fn supported_request_cannot_cross_to_a_conflicting_unavailable_declaration() {
    let fixture = Fixture::new("capability-declaration-conflict");
    let bundle = fixture.bundle("0.0.12");
    let empty = LifecycleState::default();
    let (mut supported, _) = fixture.session(&bundle, fresh_plan(&bundle, &empty));
    let unavailable = HostCapabilityDeclaration::unavailable_codex_app(
        &fixture.root,
        &fixture.project,
        "CONFLICTING_UNAVAILABLE_DECLARATION",
    )
    .unwrap();
    let mut conflicting = fixture.session_with_scope(
        &bundle,
        fresh_plan(&bundle, &empty),
        unavailable,
        HostScopeAuthority::Personal {
            marketplace: "local-harness-plugins".into(),
        },
    );
    supported.apply_confined(&empty).unwrap();
    let request = supported.take_external_effect_request().unwrap();
    let before = fixture.tree();
    assert_eq!(
        conflicting
            .consume_external_effect_request(request)
            .unwrap_err()
            .id(),
        HostLifecycleErrorId::ExternalEffectSessionMismatch
    );
    assert_eq!(fixture.tree(), before);
    assert_eq!(
        conflicting.apply_confined(&empty).unwrap_err().id(),
        HostLifecycleErrorId::HostCapabilityRejected
    );
    assert_eq!(fixture.tree(), before);
}

fn unavailable_session(
    intent: LifecycleIntent,
) -> (
    Fixture,
    crate::host_lifecycle::HostLifecycleSession,
    LifecycleState,
    String,
) {
    let fixture = Fixture::new(&format!("capability-{intent:?}"));
    let v11 = fixture.bundle("0.0.11");
    let v12 = fixture.bundle("0.0.12");
    let (bundle, observed, plan) = match intent {
        LifecycleIntent::FreshInstall => {
            let observed = LifecycleState::default();
            let plan = fresh_plan(&v12, &observed);
            (&v12, observed, plan)
        }
        LifecycleIntent::MonotonicUpdate => {
            fixture.seed(Some(&v11), Some(&v11));
            let observed = installed(&v11.authority, 2);
            let plan = lifecycle(
                &observed,
                request(
                    intent,
                    Some(v12.authority.clone()),
                    None,
                    observed.installed.as_ref(),
                    true,
                    false,
                ),
            );
            (&v12, observed, plan)
        }
        LifecycleIntent::AuthorizedRollback => {
            fixture.seed(Some(&v12), Some(&v12));
            let observed = installed(&v12.authority, 3);
            let plan = lifecycle(
                &observed,
                request(
                    intent,
                    Some(v11.authority.clone()),
                    None,
                    observed.installed.as_ref(),
                    true,
                    true,
                ),
            );
            (&v11, observed, plan)
        }
        LifecycleIntent::FailedUpdateRecovery => {
            fixture.seed(Some(&v12), Some(&v11));
            let prior = installed(&v11.authority, 3);
            let observed = LifecycleState {
                installed: Some(v12.authority.clone()),
                cache: Some(v11.authority.clone()),
                generation: 4,
                recovery_required: true,
            };
            let plan = lifecycle(
                &observed,
                request(
                    intent,
                    None,
                    Some(prior),
                    observed.installed.as_ref(),
                    true,
                    true,
                ),
            );
            (&v11, observed, plan)
        }
        LifecycleIntent::UninstallTeardown => {
            fixture.seed(Some(&v12), Some(&v12));
            let observed = installed(&v12.authority, 5);
            let plan = lifecycle(
                &observed,
                request(intent, None, None, observed.installed.as_ref(), true, false),
            );
            (&v12, observed, plan)
        }
        LifecycleIntent::StaleCacheRecovery => {
            fixture.seed(Some(&v12), None);
            let observed = LifecycleState {
                installed: Some(v12.authority.clone()),
                cache: None,
                generation: 6,
                recovery_required: false,
            };
            let plan = lifecycle(
                &observed,
                request(intent, None, None, observed.installed.as_ref(), true, false),
            );
            (&v12, observed, plan)
        }
        _ => unreachable!(),
    };
    let canary = format!("UNAVAILABLE_HOST_{intent:?}");
    let host =
        HostCapabilityDeclaration::unavailable_codex_app(&fixture.root, &fixture.project, &canary)
            .unwrap();
    let session = fixture.session_with_scope(
        bundle,
        plan,
        host,
        HostScopeAuthority::Personal {
            marketplace: "local-harness-plugins".into(),
        },
    );
    (fixture, session, observed, canary)
}

fn fresh_plan(
    bundle: &crate::support::Bundle,
    empty: &LifecycleState,
) -> crate::plugin_product::lifecycle::LifecyclePlan {
    lifecycle(
        empty,
        request(
            LifecycleIntent::FreshInstall,
            Some(bundle.authority.clone()),
            None,
            None,
            true,
            false,
        ),
    )
}
