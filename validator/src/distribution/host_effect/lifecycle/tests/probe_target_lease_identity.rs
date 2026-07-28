impl HostTargetLease for ProbeTargetLease {
    fn identity(&self) -> &ObservedTargetIdentity {
        &self.identity
    }

    fn revalidate(&mut self) -> Result<ObservedTargetIdentity, SupportedHostLifecycleError> {
        self.revalidations.fetch_add(1, Ordering::Relaxed);
        self.identity = self
            .observations
            .pop_front()
            .ok_or_else(|| lifecycle_error(SupportedHostLifecycleErrorId::TargetSubstitution))?;
        Ok(self.identity.clone())
    }
}

struct ProbeAdapter {
    platform: DescriptorExecutionPlatform,
    calls: usize,
}

impl ProbeAdapter {
    fn linux() -> Self {
        Self {
            platform: DescriptorExecutionPlatform::Linux,
            calls: 0,
        }
    }

    fn darwin() -> Self {
        Self {
            platform: DescriptorExecutionPlatform::Darwin,
            calls: 0,
        }
    }
}

impl DescriptorExecutionAdapter for ProbeAdapter {
    fn descriptor_capability(
        &mut self,
    ) -> Result<DescriptorExecutionCapability, SupportedHostLifecycleError> {
        self.calls += 1;
        DescriptorExecutionCapability::new(
            self.platform,
            match self.platform {
                DescriptorExecutionPlatform::Darwin => {
                    DescriptorExecutionPrimitive::DarwinPosixSpawnSuspendedLoadedVnode
                }
                DescriptorExecutionPlatform::FreeBsd => DescriptorExecutionPrimitive::Fexecve,
                DescriptorExecutionPlatform::Linux | DescriptorExecutionPlatform::Other => {
                    DescriptorExecutionPrimitive::ExecveAtEmptyPath
                }
            },
            "test-descriptor-adapter".to_owned(),
            "v1".to_owned(),
        )
    }
}

fn accepted<'a>(
    fixture: &Fixture,
    ledger: &'a dyn DurableHostEffectLedger,
    head: HostEffectLedgerHead,
) -> (
    SupportedHostLifecycleCoordinator<'a>,
    AcceptedHostEffect,
    HostLifecycleCustody,
) {
    let coordinator = SupportedHostLifecycleCoordinator::bind(
        "root-lifecycle-authority".to_owned(),
        "host-effect-ledger".to_owned(),
        ledger,
    )
    .unwrap();
    let pinned = fixture.pin();
    let custody = lifecycle_custody(fixture);
    let request = coordinator
        .accept(acceptance_with_custody(fixture, &pinned, head, &custody))
        .unwrap();
    (coordinator, request, custody)
}

#[test]
fn darwin_prepares_one_sealed_handoff_before_execution() {
    let fixture = Fixture::new('1');
    let ledger_root = fixture.root.join("ledger");
    let ledger =
        FileHostEffectLedger::create(&ledger_root, "host-effect-ledger".to_owned()).unwrap();
    let head = ledger.head().unwrap();
    let (coordinator, request, mut custody) = accepted(&fixture, &ledger, head);
    let mut target = ProbeTarget::stable(&fixture.target);
    let mut clock = ProbeClock::good(10_000);
    let mut adapter = ProbeAdapter::darwin();
    let handoff = coordinator
        .prepare_as_platform(
            DescriptorExecutionPlatform::Darwin,
            preparation(
                &request,
                &mut custody,
                fixture.pin(),
                &mut target,
                &mut clock,
                &mut adapter,
            ),
        )
        .unwrap();
    handoff.with_retained_authority(|capability, _, _| {
        assert_eq!(capability.platform(), DescriptorExecutionPlatform::Darwin);
        assert_eq!(
            capability.primitive(),
            DescriptorExecutionPrimitive::DarwinPosixSpawnSuspendedLoadedVnode
        );
    });
    assert!(custody.is_released());
    assert_eq!(target.acquisitions.load(Ordering::Relaxed), 1);
    assert_eq!(target.revalidations.load(Ordering::Relaxed), 2);
    assert_eq!(clock.calls, 2);
    assert_eq!(adapter.calls, 1);
    assert_eq!(ledger.head().unwrap().generation(), 2);
}

#[test]
fn current_platform_rejects_a_mismatched_descriptor_adapter_before_reservation() {
    let fixture = Fixture::new('9');
    let ledger_root = fixture.root.join("ledger-current");
    let ledger =
        FileHostEffectLedger::create(&ledger_root, "host-effect-ledger".to_owned()).unwrap();
    let head = ledger.head().unwrap();
    let (coordinator, request, mut custody) = accepted(&fixture, &ledger, head);
    let ledger_before = recursive_snapshot(&ledger_root);
    let fixture_before = recursive_snapshot(&fixture.root);
    let mut target = ProbeTarget::stable(&fixture.target);
    let mut clock = ProbeClock::good(30_000);
    let mut adapter = ProbeAdapter::linux();

    let error = coordinator
        .prepare_current(preparation(
            &request,
            &mut custody,
            fixture.pin(),
            &mut target,
            &mut clock,
            &mut adapter,
        ))
        .unwrap_err();

    assert_eq!(
        error.id(),
        SupportedHostLifecycleErrorId::DescriptorExecutionUnavailable
    );
    assert!(!custody.is_released());
    assert_eq!(target.acquisitions.load(Ordering::Relaxed), 0);
    assert_eq!(target.revalidations.load(Ordering::Relaxed), 0);
    assert_eq!(clock.calls, 0);
    assert_eq!(adapter.calls, 1);
    assert_eq!(recursive_snapshot(&ledger_root), ledger_before);
    assert_eq!(recursive_snapshot(&fixture.root), fixture_before);
}

#[test]
fn supported_protocol_prepares_an_opaque_descriptor_handoff_without_execution() {
    let fixture = Fixture::new('2');
    let ledger = RecordingLedger::new(0, d('a'));
    let (coordinator, request, mut custody) = accepted(&fixture, &ledger, ledger.observed_head());
    let mut target = ProbeTarget::stable(&fixture.target);
    let mut clock = ProbeClock::good(20_000);
    let mut adapter = ProbeAdapter::linux();
    let handoff = coordinator
        .prepare_as_platform(
            DescriptorExecutionPlatform::Linux,
            preparation(
                &request,
                &mut custody,
                fixture.pin(),
                &mut target,
                &mut clock,
                &mut adapter,
            ),
        )
        .unwrap();
    assert!(custody.is_released());
    assert!(matches!(
        custody.finalization_token(),
        Err(crate::plugin_product::lifecycle::LifecycleError::RecoveryUnavailable)
    ));
    assert_eq!(ledger.reserve_calls.load(Ordering::Relaxed), 1);
    assert_eq!(ledger.transition_calls.load(Ordering::Relaxed), 1);
    assert_eq!(target.acquisitions.load(Ordering::Relaxed), 1);
    assert_eq!(target.revalidations.load(Ordering::Relaxed), 2);
    assert_eq!(clock.calls, 2);
    assert_eq!(adapter.calls, 1);
    handoff.with_retained_authority(|capability, effect, target| {
        assert_eq!(capability.platform(), DescriptorExecutionPlatform::Linux);
        assert_eq!(
            capability.primitive(),
            DescriptorExecutionPrimitive::ExecveAtEmptyPath
        );
        assert!(capability.capability_sha256().starts_with("sha256:"));
        assert_eq!(effect.record().state(), HostEffectState::InFlight);
        assert_eq!(effect.plan().plan_sha256(), fixture.plan.plan_sha256());
        assert_eq!(target.identity(), &fixture.target);
    });
}

#[test]
fn non_descriptor_platform_is_rejected_before_authority_release() {
    let fixture = Fixture::new('a');
    let ledger = RecordingLedger::new(0, d('a'));
    let (coordinator, request, mut custody) = accepted(&fixture, &ledger, ledger.observed_head());
    let mut target = ProbeTarget::stable(&fixture.target);
    let mut clock = ProbeClock::good(40_000);
    let mut adapter = ProbeAdapter::linux();

    let error = coordinator
        .prepare_as_platform(
            DescriptorExecutionPlatform::Other,
            preparation(
                &request,
                &mut custody,
                fixture.pin(),
                &mut target,
                &mut clock,
                &mut adapter,
            ),
        )
        .unwrap_err();

    assert_eq!(
        error.id(),
        SupportedHostLifecycleErrorId::DescriptorExecutionUnavailable
    );
    assert!(!custody.is_released());
    assert_eq!(ledger.writes(), 0);
    assert_eq!(target.acquisitions.load(Ordering::Relaxed), 0);
    assert_eq!(clock.calls, 0);
    assert_eq!(adapter.calls, 0);
}
