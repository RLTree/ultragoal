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
    bundle: &crate::host_fixture::Bundle,
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
