#[test]
fn wrong_executable_target_time_and_head_fail_before_reservation() {
    let fixture = Fixture::new('3');

    let ledger = RecordingLedger::new(0, d('a'));
    let (coordinator, request, mut custody) = accepted(&fixture, &ledger, ledger.observed_head());
    let mut target = ProbeTarget::stable(&fixture.target);
    let mut clock = ProbeClock::good(30_000);
    let mut adapter = ProbeAdapter::linux();
    assert_eq!(
        coordinator
            .prepare_as_platform(
                DescriptorExecutionPlatform::Linux,
                preparation(
                    &request,
                    &mut custody,
                    fixture.pin_alternate(),
                    &mut target,
                    &mut clock,
                    &mut adapter,
                ),
            )
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::ExecutableSubstitution
    );
    assert_eq!(ledger.writes(), 0);
    assert!(!custody.is_released());

    let ledger = RecordingLedger::new(0, d('a'));
    let (coordinator, request, mut custody) = accepted(&fixture, &ledger, ledger.observed_head());
    let other = Fixture::new('4');
    let mut target = ProbeTarget {
        observations: VecDeque::from([
            other.target.clone(),
            other.target.clone(),
            other.target.clone(),
        ]),
        acquisitions: Arc::new(AtomicUsize::new(0)),
        revalidations: Arc::new(AtomicUsize::new(0)),
    };
    let mut clock = ProbeClock::good(30_000);
    let mut adapter = ProbeAdapter::linux();
    assert_eq!(
        coordinator
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
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::TargetSubstitution
    );
    assert_eq!(ledger.writes(), 0);

    let ledger = RecordingLedger::new(0, d('a'));
    let (coordinator, request, mut custody) = accepted(&fixture, &ledger, ledger.observed_head());
    let mut target = ProbeTarget::stable(&fixture.target);
    let mut clock = ProbeClock {
        samples: VecDeque::from([
            TrustedTimeSample::new("root-monotonic-clock".to_owned(), 1, 2, 40_000).unwrap(),
            TrustedTimeSample::new("root-monotonic-clock".to_owned(), 1, 1, 39_999).unwrap(),
        ]),
        calls: 0,
    };
    let mut adapter = ProbeAdapter::linux();
    assert_eq!(
        coordinator
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
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::UntrustedTime
    );
    assert_eq!(ledger.writes(), 0);

    let accepted_head = HostEffectLedgerHead::new(0, d('a')).unwrap();
    let current_ledger = RecordingLedger::new(0, d('b'));
    let (coordinator, request, mut custody) = accepted(&fixture, &current_ledger, accepted_head);
    let mut target = ProbeTarget::stable(&fixture.target);
    let mut clock = ProbeClock::good(50_000);
    let mut adapter = ProbeAdapter::linux();
    assert_eq!(
        coordinator
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
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::StaleLedgerHead
    );
    assert_eq!(current_ledger.writes(), 0);
}
