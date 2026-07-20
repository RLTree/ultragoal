struct CountingBackend {
    calls: usize,
}

impl RetainedDescriptorProcessBackend for CountingBackend {
    fn execute(
        &mut self,
        _capability: &DescriptorExecutionCapability,
        _executable: &PinnedHostExecutable,
        _command: &crate::distribution::HostCommand,
        _policy: &HostEffectExecutionPolicy,
        _cancellation: &HostEffectCancellation,
    ) -> Result<CommandCapture, BackendFailure> {
        self.calls += 1;
        Err(BackendFailure::before_start(
            HostEffectExecutorErrorId::UnsupportedPlatform,
        ))
    }
}

struct LinuxDescriptorAdapter;

impl DescriptorExecutionAdapter for LinuxDescriptorAdapter {
    fn descriptor_capability(
        &mut self,
    ) -> Result<DescriptorExecutionCapability, SupportedHostLifecycleError> {
        DescriptorExecutionCapability::new(
            DescriptorExecutionPlatform::Linux,
            DescriptorExecutionPrimitive::ExecveAtEmptyPath,
            "executor-handoff-test-adapter".to_owned(),
            "v1".to_owned(),
        )
    }
}

fn fresh_lifecycle(package: &PackageIdentity) -> AcceptedLifecyclePlan {
    AcceptedLifecyclePlan::new(
        AcceptedLifecycleOperation::FreshInstall,
        AcceptedHostState::new(0, None, false).unwrap(),
        AcceptedHostState::new(1, Some(package.clone()), false).unwrap(),
        AcceptedHostState::new(0, None, false).unwrap(),
        AcceptedRollbackPolicy::RemoveOnlyNewTarget,
        AcceptedReconciliationPolicy::ExactPostStateAndSeparateHostLayers,
    )
    .unwrap()
}

#[test]
fn executor_refuses_foreign_platform_handoff_without_backend_start() {
    let fixture = Fixture::new();
    let package = fixture.package();
    let host = HostCapabilityDeclaration::isolated(
        &fixture.home,
        &fixture.project,
        "fixture-host",
        Some(&fixture.executable),
    )
    .unwrap();
    let journey = JourneyBinding::new(package.clone(), &host, "fixture-marketplace").unwrap();
    let (scope, target, expected_target) = fixture.scope_and_target();
    let mut target_observer = target.observer();
    let ledger =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let coordinator = SupportedHostLifecycleCoordinator::bind(
        "fixture-authority".to_owned(),
        "fixture-ledger".to_owned(),
        &ledger,
    )
    .unwrap();
    let plan = HostCommandPlan::personal_install(&package, "fixture-marketplace").unwrap();
    let pinned = PinnedHostExecutable::pin(&fixture.executable).unwrap();
    let accepted = coordinator
        .accept(HostEffectAcceptanceRequest {
            package: package.clone(),
            journey,
            host,
            lifecycle: fresh_lifecycle(&package),
            scope,
            plan: &plan,
            executable: &pinned,
            expected_target,
            expected_head: ledger.head().unwrap(),
            lifecycle_record: None,
        })
        .unwrap();
    let mut custody = lifecycle_custody(&fixture, &plan);
    let mut clock = TestClock { sequence: 0 };
    let mut adapter = LinuxDescriptorAdapter;
    let handoff = coordinator
        .prepare_as_platform(
            DescriptorExecutionPlatform::Linux,
            HostEffectPreparationRequest {
                accepted: &accepted,
                custody: &mut custody,
                executable: PinnedHostExecutable::pin(&fixture.executable).unwrap(),
                target: &mut target_observer,
                clock: &mut clock,
                adapter: &mut adapter,
            },
        )
        .unwrap();
    let mut backend = CountingBackend { calls: 0 };
    let mut executor = SupportedHostEffectExecutor::new(
        &ledger,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );

    let failure = executor
        .execute_handoff(handoff, &mut clock, &HostEffectCancellation::default())
        .unwrap_err();

    assert_eq!(backend.calls, 0);
    assert_eq!(failure.id(), HostEffectExecutorErrorId::UnsupportedPlatform);
    let recovery = failure.recovery().unwrap();
    assert!(recovery.verify_binding());
    assert_eq!(
        recovery.post_reservation_ledger_classification(),
        Some(HostEffectPostReservationLedgerClassification::StillInFlight)
    );
    assert_eq!(
        recovery.originating_error_ids(),
        &[HostEffectExecutorErrorId::UnsupportedPlatform]
    );
}
