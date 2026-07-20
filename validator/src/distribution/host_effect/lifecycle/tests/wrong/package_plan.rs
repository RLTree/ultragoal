#[test]
fn wrong_package_journey_scope_capability_and_plan_are_not_accepted() {
    let fixture = Fixture::new('7');
    let other = Fixture::new('8');
    let ledger = RecordingLedger::new(0, d('a'));
    let coordinator = SupportedHostLifecycleCoordinator::bind(
        "root-lifecycle-authority".to_owned(),
        "host-effect-ledger".to_owned(),
        &ledger,
    )
    .unwrap();
    let pinned = fixture.pin();

    assert_wrong_primitive_adapters_are_rejected();

    let mut wrong_package_request = acceptance(&fixture, &pinned, ledger.observed_head());
    wrong_package_request.journey = other.journey.clone();
    let wrong_package = coordinator.accept(wrong_package_request);
    assert_eq!(
        wrong_package.unwrap_err().id(),
        SupportedHostLifecycleErrorId::InvalidAcceptedIdentity
    );

    let mut wrong_scope_request = acceptance(&fixture, &pinned, ledger.observed_head());
    wrong_scope_request.scope = other.scope.clone();
    let wrong_scope = coordinator.accept(wrong_scope_request);
    assert_eq!(
        wrong_scope.unwrap_err().id(),
        SupportedHostLifecycleErrorId::InvalidAcceptedIdentity
    );

    let unavailable = HostCapabilityDeclaration::unavailable_codex_app(
        &fixture.home,
        &fixture.project,
        "host-lifecycle-063-v1",
    )
    .unwrap();
    let unavailable_journey =
        JourneyBinding::new(fixture.package.clone(), &unavailable, "local-marketplace").unwrap();
    let unavailable_scope =
        AcceptedHostScope::personal(&unavailable_journey, "local-marketplace".to_owned()).unwrap();
    let unavailable_target = ObservedTargetIdentity::new(
        &unavailable_scope,
        7,
        HostObjectIdentity::from_metadata(&fs::symlink_metadata(&fixture.project).unwrap())
            .unwrap(),
    )
    .unwrap();
    let mut wrong_capability_request = acceptance(&fixture, &pinned, ledger.observed_head());
    wrong_capability_request.journey = unavailable_journey;
    wrong_capability_request.host = unavailable;
    wrong_capability_request.scope = unavailable_scope;
    wrong_capability_request.expected_target = unavailable_target;
    let wrong_capability = coordinator.accept(wrong_capability_request);
    assert_eq!(
        wrong_capability.unwrap_err().id(),
        SupportedHostLifecycleErrorId::InvalidAcceptedIdentity
    );

    let wrong_marketplace =
        HostCommandPlan::personal_install(&fixture.package, "other-marketplace").unwrap();
    assert_eq!(
        {
            let mut request = acceptance(&fixture, &pinned, ledger.observed_head());
            request.plan = &wrong_marketplace;
            coordinator.accept(request)
        }
        .unwrap_err()
        .id(),
        SupportedHostLifecycleErrorId::PlanSubstitution
    );

    let repository_scope =
        AcceptedHostScope::repository(&fixture.journey, "local-marketplace".to_owned()).unwrap();
    let repository_target = ObservedTargetIdentity::new(
        &repository_scope,
        7,
        HostObjectIdentity::from_metadata(&fs::symlink_metadata(&fixture.project).unwrap())
            .unwrap(),
    )
    .unwrap();
    let wrong_repository_root = fs::canonicalize(&other.project).unwrap();
    let wrong_repository_plan = HostCommandPlan::repository_install(
        &fixture.package,
        wrong_repository_root.to_str().unwrap(),
        "local-marketplace",
    )
    .unwrap();
    assert_eq!(
        {
            let mut request = acceptance(&fixture, &pinned, ledger.observed_head());
            request.scope = repository_scope.clone();
            request.plan = &wrong_repository_plan;
            request.expected_target = repository_target.clone();
            coordinator.accept(request)
        }
        .unwrap_err()
        .id(),
        SupportedHostLifecycleErrorId::InvalidAcceptedIdentity
    );
    #[cfg(unix)]
    {
        let repository_alias = fixture.root.join("repository-alias");
        std::os::unix::fs::symlink(&fixture.project, &repository_alias).unwrap();
        let aliased_repository_plan = HostCommandPlan::repository_install(
            &fixture.package,
            repository_alias.to_str().unwrap(),
            "local-marketplace",
        )
        .unwrap();
        assert_eq!(
            {
                let mut request = acceptance(&fixture, &pinned, ledger.observed_head());
                request.scope = repository_scope.clone();
                request.plan = &aliased_repository_plan;
                request.expected_target = repository_target.clone();
                coordinator.accept(request)
            }
            .unwrap_err()
            .id(),
            SupportedHostLifecycleErrorId::InvalidAcceptedIdentity
        );
    }
    let repository_root = fs::canonicalize(&fixture.project).unwrap();
    let repository_plan = HostCommandPlan::repository_install(
        &fixture.package,
        repository_root.to_str().unwrap(),
        "local-marketplace",
    )
    .unwrap();
    let mut repository_request = acceptance(&fixture, &pinned, ledger.observed_head());
    repository_request.scope = repository_scope;
    repository_request.plan = &repository_plan;
    repository_request.expected_target = repository_target;
    coordinator.accept(repository_request).unwrap();

    for operation in [
        AcceptedLifecycleOperation::IdempotentReinstall,
        AcceptedLifecycleOperation::RepeatUse,
    ] {
        let state = AcceptedHostState::new(1, Some(fixture.package.clone()), false).unwrap();
        let no_effect = AcceptedLifecyclePlan::new(
            operation,
            state.clone(),
            state.clone(),
            state,
            AcceptedRollbackPolicy::RestoreExactPreState,
            AcceptedReconciliationPolicy::ExactPostStateAndSeparateHostLayers,
        )
        .unwrap();
        assert_eq!(
            {
                let mut request = acceptance(&fixture, &pinned, ledger.observed_head());
                request.lifecycle = no_effect;
                coordinator.accept(request)
            }
            .unwrap_err()
            .id(),
            SupportedHostLifecycleErrorId::InvalidAcceptedIdentity
        );
    }

    let request = coordinator
        .accept(acceptance(&fixture, &pinned, ledger.observed_head()))
        .unwrap();
    assert_eq!(
        RootPlanCustody::bind(other.plan.clone(), &request)
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::PlanSubstitution
    );
}
