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
        crate::host_lifecycle::HostLifecycleBindRequest {
            root: mismatch.confined(),
            package_plan: &bundle.plan,
            package: &bundle.snapshot,
            lifecycle: plan,
            host: mismatch.host(),
            marketplace_plan: crate::host_fixture::marketplace_plan(&bundle),
            host_scope: HostScopeAuthority::Repository {
                repository_root: other.to_string_lossy().into_owned(),
                marketplace: "local-harness-plugins".into(),
            },
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
