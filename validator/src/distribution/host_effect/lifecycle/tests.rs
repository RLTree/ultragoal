use super::super::{
    DurableHostEffectLedger, FileHostEffectLedger, HostEffectAuthority, HostEffectLedgerError,
    HostEffectLedgerErrorId, HostEffectLedgerHead, HostEffectLedgerRecord, HostEffectReservation,
    HostEffectState, HostEffectTransition, PinnedHostExecutable,
};
use super::recovery::{
    ExpectedPublicationObjectIdentity, PublicationAcknowledgementIdentity, PublicationExpectation,
};
use super::*;
use crate::distribution::{
    HostCapabilityDeclaration, HostCommandPlan, JourneyBinding, PackageIdentity, SourceIdentity,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

fn d(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

struct Fixture {
    root: PathBuf,
    home: PathBuf,
    project: PathBuf,
    executable: PathBuf,
    alternate_executable: PathBuf,
    package: PackageIdentity,
    host: HostCapabilityDeclaration,
    journey: JourneyBinding,
    lifecycle: AcceptedLifecyclePlan,
    scope: AcceptedHostScope,
    plan: HostCommandPlan,
    target: ObservedTargetIdentity,
}

impl Fixture {
    fn new(seed: char) -> Self {
        let root = std::env::temp_dir().join(format!(
            "hul-supported-host-lifecycle-063-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let home = root.join("home");
        let project = root.join("project");
        fs::create_dir(&home).unwrap();
        fs::create_dir(&project).unwrap();
        let executable = root.join("codex");
        let alternate_executable = root.join("codex-other");
        fs::write(&executable, b"descriptor-execution-fixture-v1").unwrap();
        fs::write(&alternate_executable, b"descriptor-execution-fixture-v1").unwrap();
        #[cfg(unix)]
        {
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
            fs::set_permissions(&alternate_executable, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let source = SourceIdentity::new(
            d(seed),
            d(next_hex(seed)),
            "harness-ultragoal".to_owned(),
            "0.0.11".to_owned(),
            d('c'),
            d('d'),
        )
        .unwrap();
        let package = PackageIdentity::new(source, d('e'), d('f')).unwrap();
        let host = HostCapabilityDeclaration::isolated(
            &home,
            &project,
            "host-lifecycle-063-v1",
            Some(&executable),
        )
        .unwrap();
        let journey = JourneyBinding::new(package.clone(), &host).unwrap();
        let lifecycle = lifecycle(&package);
        let scope = AcceptedHostScope::personal(&journey, "local-marketplace".to_owned()).unwrap();
        let plan = HostCommandPlan::personal_install(&package, "local-marketplace").unwrap();
        let object =
            HostObjectIdentity::from_metadata(&fs::symlink_metadata(&project).unwrap()).unwrap();
        let target = ObservedTargetIdentity::new(&scope, 7, object).unwrap();
        Self {
            root,
            home,
            project,
            executable,
            alternate_executable,
            package,
            host,
            journey,
            lifecycle,
            scope,
            plan,
            target,
        }
    }

    fn pin(&self) -> PinnedHostExecutable {
        PinnedHostExecutable::pin(&self.executable).unwrap()
    }

    fn pin_alternate(&self) -> PinnedHostExecutable {
        PinnedHostExecutable::pin(&self.alternate_executable).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn lifecycle(package: &PackageIdentity) -> AcceptedLifecyclePlan {
    let before = AcceptedHostState::new(0, None, false).unwrap();
    let after = AcceptedHostState::new(1, Some(package.clone()), false).unwrap();
    let rollback = AcceptedHostState::new(0, None, false).unwrap();
    AcceptedLifecyclePlan::new(
        AcceptedLifecycleOperation::FreshInstall,
        before,
        after,
        rollback,
        AcceptedRollbackPolicy::RemoveOnlyNewTarget,
        AcceptedReconciliationPolicy::ExactPostStateAndSeparateHostLayers,
    )
    .unwrap()
}

fn next_hex(value: char) -> char {
    match value {
        '0'..='8' => char::from_u32(value as u32 + 1).unwrap(),
        '9' => 'a',
        'a'..='e' => char::from_u32(value as u32 + 1).unwrap(),
        _ => '1',
    }
}

struct RecordingLedger {
    inner: Mutex<LedgerState>,
    head_calls: AtomicUsize,
    reserve_calls: AtomicUsize,
    transition_calls: AtomicUsize,
}

struct LedgerState {
    head: HostEffectLedgerHead,
    records: BTreeMap<String, HostEffectLedgerRecord>,
}

impl RecordingLedger {
    fn new(generation: u64, head_sha256: String) -> Self {
        Self {
            inner: Mutex::new(LedgerState {
                head: HostEffectLedgerHead::new(generation, head_sha256).unwrap(),
                records: BTreeMap::new(),
            }),
            head_calls: AtomicUsize::new(0),
            reserve_calls: AtomicUsize::new(0),
            transition_calls: AtomicUsize::new(0),
        }
    }

    fn observed_head(&self) -> HostEffectLedgerHead {
        self.inner.lock().unwrap().head.clone()
    }

    fn writes(&self) -> usize {
        self.reserve_calls.load(Ordering::Relaxed) + self.transition_calls.load(Ordering::Relaxed)
    }
}

impl DurableHostEffectLedger for RecordingLedger {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError> {
        self.head_calls.fetch_add(1, Ordering::Relaxed);
        Ok(self.inner.lock().unwrap().head.clone())
    }

    fn reserve(
        &self,
        reservation: HostEffectReservation,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        self.reserve_calls.fetch_add(1, Ordering::Relaxed);
        let mut inner = self.inner.lock().unwrap();
        if reservation.expected_head_sha256() != inner.head.head_sha256() {
            return Err(HostEffectLedgerError::new(
                HostEffectLedgerErrorId::StaleHead,
            ));
        }
        let prior = inner.head.clone();
        let next = HostEffectLedgerHead::new(
            prior.generation() + 1,
            digest(format!("reserve:{}", reservation.permit_id()).as_bytes()),
        )?;
        let record = HostEffectLedgerRecord {
            reservation: reservation.clone(),
            state: HostEffectState::Reserved,
            record_sha256: digest(format!("record:{}", reservation.permit_id()).as_bytes()),
            prior_head: prior,
            current_head: next.clone(),
            outcome_sha256: None,
        };
        inner.head = next;
        inner
            .records
            .insert(reservation.permit_id().to_owned(), record.clone());
        Ok(record)
    }

    fn transition(
        &self,
        transition: HostEffectTransition,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        self.transition_calls.fetch_add(1, Ordering::Relaxed);
        let mut inner = self.inner.lock().unwrap();
        if transition.expected_head != inner.head {
            return Err(HostEffectLedgerError::new(
                HostEffectLedgerErrorId::StaleHead,
            ));
        }
        let current = inner
            .records
            .get(&transition.permit_id)
            .cloned()
            .ok_or_else(|| HostEffectLedgerError::new(HostEffectLedgerErrorId::InvalidRecord))?;
        if current.state() != transition.expected_state
            || transition.next_state != HostEffectState::InFlight
        {
            return Err(HostEffectLedgerError::new(
                HostEffectLedgerErrorId::InvalidTransition,
            ));
        }
        let next = HostEffectLedgerHead::new(
            inner.head.generation() + 1,
            digest(format!("transition:{}", transition.permit_id).as_bytes()),
        )?;
        let record = HostEffectLedgerRecord {
            reservation: current.reservation().clone(),
            state: transition.next_state,
            record_sha256: digest(format!("in-flight:{}", transition.permit_id).as_bytes()),
            prior_head: inner.head.clone(),
            current_head: next.clone(),
            outcome_sha256: transition.outcome_sha256,
        };
        inner.head = next;
        inner.records.insert(transition.permit_id, record.clone());
        Ok(record)
    }

    fn read(
        &self,
        permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError> {
        Ok(self.inner.lock().unwrap().records.get(permit_id).cloned())
    }
}

struct ProbeClock {
    samples: VecDeque<TrustedTimeSample>,
    calls: usize,
}

impl ProbeClock {
    fn good(start: u64) -> Self {
        Self {
            samples: VecDeque::from([
                TrustedTimeSample::new("root-monotonic-clock".to_owned(), 1, 1, start).unwrap(),
                TrustedTimeSample::new("root-monotonic-clock".to_owned(), 1, 2, start + 1).unwrap(),
            ]),
            calls: 0,
        }
    }
}

impl RootTrustedClock for ProbeClock {
    fn sample(&mut self) -> Result<TrustedTimeSample, SupportedHostLifecycleError> {
        self.calls += 1;
        self.samples
            .pop_front()
            .ok_or_else(|| lifecycle_error(SupportedHostLifecycleErrorId::UntrustedTime))
    }
}

struct ProbeTarget {
    observations: VecDeque<ObservedTargetIdentity>,
    acquisitions: Arc<AtomicUsize>,
    revalidations: Arc<AtomicUsize>,
}

impl ProbeTarget {
    fn stable(target: &ObservedTargetIdentity) -> Self {
        Self {
            observations: VecDeque::from([target.clone(), target.clone(), target.clone()]),
            acquisitions: Arc::new(AtomicUsize::new(0)),
            revalidations: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl HostTargetObserver for ProbeTarget {
    fn acquire(
        &mut self,
        _expected: &ObservedTargetIdentity,
    ) -> Result<Box<dyn HostTargetLease>, SupportedHostLifecycleError> {
        self.acquisitions.fetch_add(1, Ordering::Relaxed);
        let identity = self
            .observations
            .pop_front()
            .ok_or_else(|| lifecycle_error(SupportedHostLifecycleErrorId::TargetSubstitution))?;
        Ok(Box::new(ProbeTargetLease {
            identity,
            observations: std::mem::take(&mut self.observations),
            revalidations: Arc::clone(&self.revalidations),
        }))
    }
}

struct ProbeTargetLease {
    identity: ObservedTargetIdentity,
    observations: VecDeque<ObservedTargetIdentity>,
    revalidations: Arc<AtomicUsize>,
}

struct ExecutableRaceTarget {
    target: ObservedTargetIdentity,
    executable: PathBuf,
}

impl HostTargetObserver for ExecutableRaceTarget {
    fn acquire(
        &mut self,
        _expected: &ObservedTargetIdentity,
    ) -> Result<Box<dyn HostTargetLease>, SupportedHostLifecycleError> {
        Ok(Box::new(ExecutableRaceLease {
            target: self.target.clone(),
            executable: self.executable.clone(),
            revalidations: 0,
        }))
    }
}

struct ExecutableRaceLease {
    target: ObservedTargetIdentity,
    executable: PathBuf,
    revalidations: usize,
}

impl HostTargetLease for ExecutableRaceLease {
    fn identity(&self) -> &ObservedTargetIdentity {
        &self.target
    }

    fn revalidate(&mut self) -> Result<ObservedTargetIdentity, SupportedHostLifecycleError> {
        self.revalidations += 1;
        if self.revalidations == 2 {
            fs::write(
                &self.executable,
                b"descriptor-executable-raced-after-inflight",
            )
            .unwrap();
        }
        Ok(self.target.clone())
    }
}

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
}

impl DescriptorExecutionAdapter for ProbeAdapter {
    fn descriptor_capability(
        &mut self,
    ) -> Result<DescriptorExecutionCapability, SupportedHostLifecycleError> {
        self.calls += 1;
        DescriptorExecutionCapability::new(
            self.platform,
            if self.platform == DescriptorExecutionPlatform::FreeBsd {
                DescriptorExecutionPrimitive::Fexecve
            } else {
                DescriptorExecutionPrimitive::ExecveAtEmptyPath
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
    RootPlanCustody,
) {
    let coordinator = SupportedHostLifecycleCoordinator::bind(
        "root-lifecycle-authority".to_owned(),
        "host-effect-ledger".to_owned(),
        ledger,
    )
    .unwrap();
    let pinned = fixture.pin();
    let request = coordinator
        .accept(
            fixture.package.clone(),
            fixture.journey.clone(),
            fixture.host.clone(),
            fixture.lifecycle.clone(),
            fixture.scope.clone(),
            &fixture.plan,
            &pinned,
            fixture.target.clone(),
            head,
        )
        .unwrap();
    let custody = RootPlanCustody::bind(fixture.plan.clone(), &request).unwrap();
    (coordinator, request, custody)
}

#[test]
fn darwin_is_repeatedly_unsupported_before_every_observable_boundary() {
    let fixture = Fixture::new('1');
    let ledger_root = fixture.root.join("ledger");
    let ledger =
        FileHostEffectLedger::create(&ledger_root, "host-effect-ledger".to_owned()).unwrap();
    let head = ledger.head().unwrap();
    let (coordinator, request, mut custody) = accepted(&fixture, &ledger, head);
    let ledger_before = recursive_snapshot(&ledger_root);
    let fixture_before = recursive_snapshot(&fixture.root);
    let mut target = ProbeTarget::stable(&fixture.target);
    let mut clock = ProbeClock::good(10_000);
    let mut adapter = ProbeAdapter::linux();

    for _ in 0..2 {
        let error = coordinator
            .prepare_as_platform(
                DescriptorExecutionPlatform::Darwin,
                &request,
                &mut custody,
                fixture.pin(),
                &mut target,
                &mut clock,
                &mut adapter,
            )
            .unwrap_err();
        assert_eq!(
            error.id(),
            SupportedHostLifecycleErrorId::UnsupportedPlatform
        );
    }

    assert!(!custody.is_released());
    assert_eq!(target.acquisitions.load(Ordering::Relaxed), 0);
    assert_eq!(target.revalidations.load(Ordering::Relaxed), 0);
    assert_eq!(clock.calls, 0);
    assert_eq!(adapter.calls, 0);
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
            &request,
            &mut custody,
            fixture.pin(),
            &mut target,
            &mut clock,
            &mut adapter,
        )
        .unwrap();
    assert!(custody.is_released());
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
                &request,
                &mut custody,
                fixture.pin_alternate(),
                &mut target,
                &mut clock,
                &mut adapter,
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
                &request,
                &mut custody,
                fixture.pin(),
                &mut target,
                &mut clock,
                &mut adapter,
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
                &request,
                &mut custody,
                fixture.pin(),
                &mut target,
                &mut clock,
                &mut adapter,
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
                &request,
                &mut custody,
                fixture.pin(),
                &mut target,
                &mut clock,
                &mut adapter,
            )
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::StaleLedgerHead
    );
    assert_eq!(current_ledger.writes(), 0);
}

#[test]
fn target_race_and_coordinator_key_ledger_or_authority_substitution_fail_closed() {
    let fixture = Fixture::new('5');
    let ledger = RecordingLedger::new(0, d('a'));
    let (coordinator, request, mut custody) = accepted(&fixture, &ledger, ledger.observed_head());
    let other = Fixture::new('6');
    let mut target = ProbeTarget {
        observations: VecDeque::from([fixture.target.clone(), other.target.clone()]),
        acquisitions: Arc::new(AtomicUsize::new(0)),
        revalidations: Arc::new(AtomicUsize::new(0)),
    };
    let mut clock = ProbeClock::good(60_000);
    let mut adapter = ProbeAdapter::linux();
    assert_eq!(
        coordinator
            .prepare_as_platform(
                DescriptorExecutionPlatform::Linux,
                &request,
                &mut custody,
                fixture.pin(),
                &mut target,
                &mut clock,
                &mut adapter,
            )
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::TargetRace
    );
    assert_eq!(ledger.writes(), 0);

    let mut target = ProbeTarget {
        observations: VecDeque::from([
            fixture.target.clone(),
            fixture.target.clone(),
            other.target.clone(),
        ]),
        acquisitions: Arc::new(AtomicUsize::new(0)),
        revalidations: Arc::new(AtomicUsize::new(0)),
    };
    let mut clock = ProbeClock::good(65_000);
    let mut adapter = ProbeAdapter::linux();
    assert_eq!(
        coordinator
            .prepare_as_platform(
                DescriptorExecutionPlatform::Linux,
                &request,
                &mut custody,
                fixture.pin(),
                &mut target,
                &mut clock,
                &mut adapter,
            )
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::TargetRace
    );
    assert_eq!(ledger.writes(), 2);
    assert!(!custody.is_released());

    for (issuer, replacement_ledger) in [
        (
            "root-lifecycle-authority",
            &ledger as &dyn DurableHostEffectLedger,
        ),
        (
            "other-root-authority",
            &ledger as &dyn DurableHostEffectLedger,
        ),
    ] {
        let replacement = SupportedHostLifecycleCoordinator::bind(
            issuer.to_owned(),
            "host-effect-ledger".to_owned(),
            replacement_ledger,
        )
        .unwrap();
        let mut target = ProbeTarget::stable(&fixture.target);
        let mut clock = ProbeClock::good(70_000);
        let mut adapter = ProbeAdapter::linux();
        assert_eq!(
            replacement
                .prepare_as_platform(
                    DescriptorExecutionPlatform::Linux,
                    &request,
                    &mut custody,
                    fixture.pin(),
                    &mut target,
                    &mut clock,
                    &mut adapter,
                )
                .unwrap_err()
                .id(),
            SupportedHostLifecycleErrorId::CoordinatorSubstitution
        );
    }

    let other_ledger = RecordingLedger::new(0, d('a'));
    let replacement = SupportedHostLifecycleCoordinator::bind(
        "root-lifecycle-authority".to_owned(),
        "host-effect-ledger".to_owned(),
        &other_ledger,
    )
    .unwrap();
    let mut target = ProbeTarget::stable(&fixture.target);
    let mut clock = ProbeClock::good(80_000);
    let mut adapter = ProbeAdapter::linux();
    assert_eq!(
        replacement
            .prepare_as_platform(
                DescriptorExecutionPlatform::Linux,
                &request,
                &mut custody,
                fixture.pin(),
                &mut target,
                &mut clock,
                &mut adapter,
            )
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::CoordinatorSubstitution
    );
    assert_eq!(other_ledger.writes(), 0);
}

#[test]
fn final_executable_race_after_inflight_retains_plan_custody() {
    let fixture = Fixture::new('7');
    let ledger = RecordingLedger::new(0, d('a'));
    let (coordinator, request, mut custody) = accepted(&fixture, &ledger, ledger.observed_head());
    let mut target = ExecutableRaceTarget {
        target: fixture.target.clone(),
        executable: fixture.executable.clone(),
    };
    let mut clock = ProbeClock::good(85_000);
    let mut adapter = ProbeAdapter::linux();

    assert_eq!(
        coordinator
            .prepare_as_platform(
                DescriptorExecutionPlatform::Linux,
                &request,
                &mut custody,
                fixture.pin(),
                &mut target,
                &mut clock,
                &mut adapter,
            )
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::HandoffConstructionFailed
    );
    assert_eq!(ledger.writes(), 2);
    assert!(!custody.is_released());
}

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

    for (platform, primitive) in [
        (
            DescriptorExecutionPlatform::Linux,
            DescriptorExecutionPrimitive::Fexecve,
        ),
        (
            DescriptorExecutionPlatform::FreeBsd,
            DescriptorExecutionPrimitive::ExecveAtEmptyPath,
        ),
    ] {
        assert_eq!(
            DescriptorExecutionCapability::new(
                platform,
                primitive,
                "wrong-primitive-adapter".to_owned(),
                "v1".to_owned(),
            )
            .unwrap_err()
            .id(),
            SupportedHostLifecycleErrorId::DescriptorExecutionUnavailable
        );
    }

    let wrong_package = coordinator.accept(
        fixture.package.clone(),
        other.journey.clone(),
        fixture.host.clone(),
        fixture.lifecycle.clone(),
        fixture.scope.clone(),
        &fixture.plan,
        &pinned,
        fixture.target.clone(),
        ledger.observed_head(),
    );
    assert_eq!(
        wrong_package.unwrap_err().id(),
        SupportedHostLifecycleErrorId::InvalidAcceptedIdentity
    );

    let wrong_scope = coordinator.accept(
        fixture.package.clone(),
        fixture.journey.clone(),
        fixture.host.clone(),
        fixture.lifecycle.clone(),
        other.scope.clone(),
        &fixture.plan,
        &pinned,
        fixture.target.clone(),
        ledger.observed_head(),
    );
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
    let unavailable_journey = JourneyBinding::new(fixture.package.clone(), &unavailable).unwrap();
    let unavailable_scope =
        AcceptedHostScope::personal(&unavailable_journey, "local-marketplace".to_owned()).unwrap();
    let unavailable_target = ObservedTargetIdentity::new(
        &unavailable_scope,
        7,
        HostObjectIdentity::from_metadata(&fs::symlink_metadata(&fixture.project).unwrap())
            .unwrap(),
    )
    .unwrap();
    let wrong_capability = coordinator.accept(
        fixture.package.clone(),
        unavailable_journey,
        unavailable,
        fixture.lifecycle.clone(),
        unavailable_scope,
        &fixture.plan,
        &pinned,
        unavailable_target,
        ledger.observed_head(),
    );
    assert_eq!(
        wrong_capability.unwrap_err().id(),
        SupportedHostLifecycleErrorId::InvalidAcceptedIdentity
    );

    let wrong_marketplace =
        HostCommandPlan::personal_install(&fixture.package, "other-marketplace").unwrap();
    assert_eq!(
        coordinator
            .accept(
                fixture.package.clone(),
                fixture.journey.clone(),
                fixture.host.clone(),
                fixture.lifecycle.clone(),
                fixture.scope.clone(),
                &wrong_marketplace,
                &pinned,
                fixture.target.clone(),
                ledger.observed_head(),
            )
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
        coordinator
            .accept(
                fixture.package.clone(),
                fixture.journey.clone(),
                fixture.host.clone(),
                fixture.lifecycle.clone(),
                repository_scope.clone(),
                &wrong_repository_plan,
                &pinned,
                repository_target.clone(),
                ledger.observed_head(),
            )
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
            coordinator
                .accept(
                    fixture.package.clone(),
                    fixture.journey.clone(),
                    fixture.host.clone(),
                    fixture.lifecycle.clone(),
                    repository_scope.clone(),
                    &aliased_repository_plan,
                    &pinned,
                    repository_target.clone(),
                    ledger.observed_head(),
                )
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
    coordinator
        .accept(
            fixture.package.clone(),
            fixture.journey.clone(),
            fixture.host.clone(),
            fixture.lifecycle.clone(),
            repository_scope,
            &repository_plan,
            &pinned,
            repository_target,
            ledger.observed_head(),
        )
        .unwrap();

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
            coordinator
                .accept(
                    fixture.package.clone(),
                    fixture.journey.clone(),
                    fixture.host.clone(),
                    no_effect,
                    fixture.scope.clone(),
                    &fixture.plan,
                    &pinned,
                    fixture.target.clone(),
                    ledger.observed_head(),
                )
                .unwrap_err()
                .id(),
            SupportedHostLifecycleErrorId::InvalidAcceptedIdentity
        );
    }

    let request = coordinator
        .accept(
            fixture.package.clone(),
            fixture.journey.clone(),
            fixture.host.clone(),
            fixture.lifecycle.clone(),
            fixture.scope.clone(),
            &fixture.plan,
            &pinned,
            fixture.target.clone(),
            ledger.observed_head(),
        )
        .unwrap();
    assert_eq!(
        RootPlanCustody::bind(other.plan.clone(), &request)
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::PlanSubstitution
    );
}

#[test]
fn every_complete_permit_binding_dimension_changes_the_canonical_binding() {
    let fixture = Fixture::new('9');
    let ledger = RecordingLedger::new(0, d('a'));
    let (_coordinator, request, _custody) = accepted(&fixture, &ledger, ledger.observed_head());
    let baseline = request
        .derive_binding(100_000, 100_001, &ledger.observed_head())
        .unwrap();
    let baseline_digest = digest(&serde_json::to_vec(&baseline).unwrap());
    let mutations: &[fn(&mut super::super::HostEffectPermitBinding)] = &[
        |row| row.context_id = d('b'),
        |row| row.candidate_id = d('b'),
        |row| row.package_identity_sha256 = d('b'),
        |row| row.journey_binding_sha256 = d('b'),
        |row| row.session_issuance_sha256 = d('b'),
        |row| row.lifecycle_plan_sha256 = d('b'),
        |row| row.lifecycle_intent = "authorized-rollback".to_owned(),
        |row| row.expected_pre_state_sha256 = d('b'),
        |row| row.expected_post_state_sha256 = d('b'),
        |row| row.rollback_policy_sha256 = d('b'),
        |row| row.reconciliation_policy_sha256 = d('b'),
        |row| row.host_scope_sha256 = d('b'),
        |row| row.host_capability_sha256 = d('b'),
        |row| row.required_capabilities_sha256 = d('b'),
        |row| row.external_request_sha256 = d('b'),
        |row| row.command_plan_sha256 = d('b'),
        |row| row.argv_sha256 = d('b'),
        |row| row.executable_identity_sha256 = d('b'),
        |row| row.target_identity_sha256 = d('b'),
        |row| row.target_generation += 1,
        |row| row.issued_at_unix_ms += 1,
        |row| row.expires_at_unix_ms += 1,
        |row| row.expected_head_sha256 = d('b'),
        |row| row.decision = super::super::HostEffectDecision::Refuse,
    ];
    assert_eq!(mutations.len(), 24);
    let authority = HostEffectAuthority::generate(
        "root-lifecycle-authority".to_owned(),
        "host-effect-ledger".to_owned(),
    )
    .unwrap();
    authority.issue(baseline.clone()).unwrap();
    for mutate in mutations {
        let mut changed = baseline.clone();
        mutate(&mut changed);
        assert_ne!(
            digest(&serde_json::to_vec(&changed).unwrap()),
            baseline_digest
        );
    }
}

#[test]
fn exact_publication_identities_classify_every_recoverable_state() {
    let prior = d('1');
    let next = d('2');
    let head = recovery_head(7, 'a');
    let prior_expectation = publication_expectation(expected_regular(
        "ledger.json",
        100,
        0o100400,
        prior.clone(),
        true,
    ));
    let missing_expectation = publication_expectation(expected_missing("ledger.json"));
    let target_prior = regular_with_mode("ledger.json", 10, 100, 0o100400, Some(prior), true);
    let target_next = regular_with_mode("ledger.json", 10, 200, 0o100400, Some(next.clone()), true);
    let target_missing =
        PublicationObjectObservation::missing("ledger.json".to_owned(), 10).unwrap();
    let temp_empty = regular_with_mode(
        ".ledger.json.0123456789abcdef.tmp",
        10,
        0,
        0o100400,
        None,
        false,
    );
    let temp_unflushed = regular_with_mode(
        ".ledger.json.0123456789abcdef.tmp",
        10,
        200,
        0o100400,
        Some(next.clone()),
        false,
    );
    let temp_synced = regular_with_mode(
        ".ledger.json.0123456789abcdef.tmp",
        10,
        200,
        0o100400,
        Some(next),
        true,
    );
    let acknowledgement =
        PublicationAcknowledgementIdentity::new(&prior_expectation, &head).unwrap();

    let cases = [
        (
            inventory(
                target_missing.clone(),
                vec![],
                missing_expectation.clone(),
                head.clone(),
                None,
            ),
            PublicationClassificationId::CleanPriorState,
        ),
        (
            inventory(
                target_prior.clone(),
                vec![],
                prior_expectation.clone(),
                head.clone(),
                None,
            ),
            PublicationClassificationId::CleanPriorState,
        ),
        (
            inventory(
                target_missing,
                vec![temp_empty],
                missing_expectation,
                head.clone(),
                None,
            ),
            PublicationClassificationId::InterruptedBeforeTempWrite,
        ),
        (
            inventory(
                target_prior.clone(),
                vec![temp_unflushed],
                prior_expectation.clone(),
                head.clone(),
                None,
            ),
            PublicationClassificationId::InterruptedDuringTempFsync,
        ),
        (
            inventory(
                target_prior.clone(),
                vec![temp_synced.clone()],
                prior_expectation.clone(),
                head.clone(),
                None,
            ),
            PublicationClassificationId::InterruptedBeforeRename,
        ),
        (
            inventory(
                target_next.clone(),
                vec![],
                prior_expectation.clone(),
                head.clone(),
                None,
            ),
            PublicationClassificationId::CommittedBeforeAcknowledgement,
        ),
        (
            inventory(
                target_next.clone(),
                vec![],
                prior_expectation.clone(),
                head.clone(),
                Some(acknowledgement),
            ),
            PublicationClassificationId::AcknowledgedCommitted,
        ),
        (
            inventory(
                target_next,
                vec![temp_synced],
                prior_expectation,
                head,
                None,
            ),
            PublicationClassificationId::OrphanedTemporaryObject,
        ),
    ];
    for (observation, expected) in cases {
        assert_eq!(observation.classify().unwrap().id(), expected);
    }
}

#[test]
fn expected_publication_identity_binds_every_field_and_rejects_unsafe_shapes() {
    let baseline =
        publication_expectation(expected_regular("ledger.json", 100, 0o100400, d('1'), true));
    let baseline_identity = baseline.publication_identity_sha256().to_owned();
    let variants = [
        publication_expectation_with_effect(
            d('f'),
            expected_regular("ledger.json", 100, 0o100400, d('1'), true),
        ),
        publication_expectation(expected_missing("ledger.json")),
        publication_expectation(expected_regular("ledger.json", 101, 0o100400, d('1'), true)),
        publication_expectation(expected_regular("ledger.json", 100, 0o100440, d('1'), true)),
        publication_expectation(expected_regular("ledger.json", 100, 0o100400, d('3'), true)),
        publication_expectation_exact(
            d('e'),
            expected_regular("ledger.json", 100, 0o100400, d('1'), true),
            "ledger.json",
            ".ledger.json.fedcba9876543210.tmp",
            200,
            0o100400,
            d('2'),
            true,
        )
        .unwrap(),
        publication_expectation_exact(
            d('e'),
            expected_regular("ledger.json", 100, 0o100400, d('1'), true),
            "ledger.json",
            ".ledger.json.0123456789abcdef.tmp",
            201,
            0o100400,
            d('2'),
            true,
        )
        .unwrap(),
        publication_expectation_exact(
            d('e'),
            expected_regular("ledger.json", 100, 0o100400, d('1'), true),
            "ledger.json",
            ".ledger.json.0123456789abcdef.tmp",
            200,
            0o100440,
            d('2'),
            true,
        )
        .unwrap(),
        publication_expectation_exact(
            d('e'),
            expected_regular("ledger.json", 100, 0o100400, d('1'), true),
            "ledger.json",
            ".ledger.json.0123456789abcdef.tmp",
            200,
            0o100400,
            d('4'),
            true,
        )
        .unwrap(),
        publication_expectation_exact(
            d('e'),
            expected_regular("state.json", 100, 0o100400, d('1'), true),
            "state.json",
            ".state.json.0123456789abcdef.tmp",
            200,
            0o100400,
            d('2'),
            true,
        )
        .unwrap(),
    ];
    for changed in variants {
        assert_ne!(
            changed.publication_identity_sha256(),
            baseline_identity.as_str()
        );
    }

    for invalid in [
        ExpectedPublicationObjectIdentity::regular(
            "ledger.json".to_owned(),
            100,
            0o100400,
            1,
            "not-a-digest".to_owned(),
            true,
        ),
        ExpectedPublicationObjectIdentity::regular(
            "ledger.json".to_owned(),
            100,
            0o040400,
            1,
            d('1'),
            true,
        ),
        ExpectedPublicationObjectIdentity::regular(
            "ledger.json".to_owned(),
            100,
            0o100400,
            2,
            d('1'),
            true,
        ),
        ExpectedPublicationObjectIdentity::missing("../ledger.json".to_owned()),
    ] {
        assert_eq!(
            invalid.unwrap_err().id(),
            SupportedHostLifecycleErrorId::RecoveryUnsafe
        );
    }

    let exact_next = expected_regular("ledger.json", 200, 0o100400, d('2'), true);
    let exact_temp = expected_regular(
        ".ledger.json.0123456789abcdef.tmp",
        200,
        0o100400,
        d('2'),
        true,
    );
    let invalid_expectations = [
        PublicationExpectation::new(
            "not-a-digest".to_owned(),
            expected_missing("ledger.json"),
            exact_next.clone(),
            exact_temp.clone(),
        ),
        PublicationExpectation::new(
            d('e'),
            expected_regular("ledger.json", 100, 0o100600, d('1'), true),
            exact_next.clone(),
            exact_temp.clone(),
        ),
        PublicationExpectation::new(
            d('e'),
            expected_regular("ledger.json", 100, 0o100400, d('1'), false),
            exact_next.clone(),
            exact_temp.clone(),
        ),
        PublicationExpectation::new(
            d('e'),
            expected_missing("other.json"),
            exact_next.clone(),
            exact_temp.clone(),
        ),
        PublicationExpectation::new(
            d('e'),
            expected_missing("ledger.json"),
            expected_regular("ledger.json", 200, 0o100600, d('2'), true),
            expected_regular(
                ".ledger.json.0123456789abcdef.tmp",
                200,
                0o100600,
                d('2'),
                true,
            ),
        ),
        PublicationExpectation::new(
            d('e'),
            expected_missing("ledger.json"),
            expected_regular("ledger.json", 200, 0o100400, d('2'), false),
            expected_regular(
                ".ledger.json.0123456789abcdef.tmp",
                200,
                0o100400,
                d('2'),
                false,
            ),
        ),
        PublicationExpectation::new(
            d('e'),
            expected_missing("ledger.json"),
            exact_next.clone(),
            expected_regular(
                ".ledger.json.0123456789abcdef.tmp",
                201,
                0o100400,
                d('2'),
                true,
            ),
        ),
        PublicationExpectation::new(
            d('e'),
            expected_missing("ledger.json"),
            exact_next,
            expected_regular(
                ".ledger.json.not-canonical.tmp",
                200,
                0o100400,
                d('2'),
                true,
            ),
        ),
    ];
    for invalid in invalid_expectations {
        assert_eq!(
            invalid.unwrap_err().id(),
            SupportedHostLifecycleErrorId::RecoveryUnsafe
        );
    }
}

#[test]
fn target_and_temporary_exactness_rejects_every_identity_substitution() {
    let expectation =
        publication_expectation(expected_regular("ledger.json", 100, 0o100400, d('1'), true));
    let head = recovery_head(7, 'a');
    let acknowledgement = PublicationAcknowledgementIdentity::new(&expectation, &head).unwrap();
    let target_mutations = [
        regular_with_mode("ledger.json", 10, 100, 0o100400, Some(d('9')), true),
        regular_with_mode("ledger.json", 10, 101, 0o100400, Some(d('1')), true),
        regular_with_mode("ledger.json", 10, 100, 0o100600, Some(d('1')), true),
        regular_with_mode("ledger.json", 10, 100, 0o100400, Some(d('1')), false),
    ];
    for changed in target_mutations {
        assert_eq!(
            inventory(
                changed.clone(),
                vec![],
                expectation.clone(),
                head.clone(),
                None,
            )
            .classify()
            .unwrap()
            .id(),
            PublicationClassificationId::UnknownState
        );
        assert_eq!(
            inventory(
                changed,
                vec![],
                expectation.clone(),
                head.clone(),
                Some(acknowledgement.clone()),
            )
            .classify()
            .unwrap()
            .id(),
            PublicationClassificationId::FalsePassReceipt
        );
    }

    let next_mutations = [
        regular_with_mode("ledger.json", 10, 200, 0o100400, Some(d('8')), true),
        regular_with_mode("ledger.json", 10, 201, 0o100400, Some(d('2')), true),
        regular_with_mode("ledger.json", 10, 200, 0o100600, Some(d('2')), true),
        regular_with_mode("ledger.json", 10, 200, 0o100400, Some(d('2')), false),
    ];
    for changed in next_mutations {
        assert_eq!(
            inventory(
                changed.clone(),
                vec![],
                expectation.clone(),
                head.clone(),
                None,
            )
            .classify()
            .unwrap()
            .id(),
            PublicationClassificationId::UnknownState
        );
        assert_eq!(
            inventory(
                changed,
                vec![],
                expectation.clone(),
                head.clone(),
                Some(acknowledgement.clone()),
            )
            .classify()
            .unwrap()
            .id(),
            PublicationClassificationId::FalsePassReceipt
        );
    }

    let target_prior = regular_with_mode("ledger.json", 10, 100, 0o100400, Some(d('1')), true);
    for changed in [
        regular_with_mode(
            ".ledger.json.0123456789abcdef.tmp",
            10,
            200,
            0o100600,
            Some(d('2')),
            true,
        ),
        regular_with_mode(
            ".ledger.json.0123456789abcdef.tmp",
            10,
            201,
            0o100400,
            Some(d('2')),
            true,
        ),
        regular_with_mode(
            ".ledger.json.0123456789abcdef.tmp",
            10,
            200,
            0o100400,
            Some(d('8')),
            true,
        ),
        regular_with_mode(
            ".ledger.json.fedcba9876543210.tmp",
            10,
            200,
            0o100400,
            Some(d('2')),
            true,
        ),
    ] {
        assert_eq!(
            inventory(
                target_prior.clone(),
                vec![changed],
                expectation.clone(),
                head.clone(),
                None,
            )
            .classify()
            .unwrap()
            .id(),
            PublicationClassificationId::OrphanedTemporaryObject
        );
    }

    for name in ["ledger.json", ".ledger.json.0123456789abcdef.tmp"] {
        assert_eq!(
            PublicationObjectObservation::new(
                name.to_owned(),
                PublicationObjectKind::Regular,
                200,
                0o100400,
                2,
                Some(d('2')),
                10,
                true,
            )
            .unwrap_err()
            .id(),
            SupportedHostLifecycleErrorId::RecoveryUnsafe
        );
    }
}

#[test]
fn acknowledgements_reject_effect_publication_head_and_wire_substitution() {
    let expectation =
        publication_expectation(expected_regular("ledger.json", 100, 0o100400, d('1'), true));
    let target_next = regular_with_mode("ledger.json", 10, 200, 0o100400, Some(d('2')), true);
    let head = recovery_head(7, 'a');
    let acknowledgement = PublicationAcknowledgementIdentity::new(&expectation, &head).unwrap();
    let canonical = serde_json::to_vec(&acknowledgement).unwrap();
    let decoded = PublicationAcknowledgementIdentity::from_canonical_json(&canonical).unwrap();
    assert_eq!(
        inventory(
            target_next.clone(),
            vec![],
            expectation.clone(),
            head.clone(),
            Some(decoded),
        )
        .classify()
        .unwrap()
        .id(),
        PublicationClassificationId::AcknowledgedCommitted
    );

    let foreign_effect = publication_expectation_with_effect(
        d('f'),
        expected_regular("ledger.json", 100, 0o100400, d('1'), true),
    );
    let foreign_publication =
        publication_expectation(expected_regular("ledger.json", 99, 0o100400, d('3'), true));
    let substitutions = [
        PublicationAcknowledgementIdentity::new(&foreign_effect, &head).unwrap(),
        PublicationAcknowledgementIdentity::new(&foreign_publication, &head).unwrap(),
        PublicationAcknowledgementIdentity::new(&expectation, &recovery_head(6, 'a')).unwrap(),
        PublicationAcknowledgementIdentity::new(&expectation, &recovery_head(7, 'b')).unwrap(),
    ];
    for changed in substitutions {
        assert_eq!(
            inventory(
                target_next.clone(),
                vec![],
                expectation.clone(),
                head.clone(),
                Some(changed),
            )
            .classify()
            .unwrap()
            .id(),
            PublicationClassificationId::FalsePassReceipt
        );
    }

    let mut unknown = serde_json::to_value(&acknowledgement).unwrap();
    unknown
        .as_object_mut()
        .unwrap()
        .insert("unknown".to_owned(), serde_json::Value::Bool(true));
    let mut missing = serde_json::to_value(&acknowledgement).unwrap();
    missing
        .as_object_mut()
        .unwrap()
        .remove("publication_identity_sha256");
    let mut extra_head = serde_json::to_value(&acknowledgement).unwrap();
    extra_head["ledger_head"]
        .as_object_mut()
        .unwrap()
        .insert("extra".to_owned(), serde_json::Value::Bool(true));
    let mut noncanonical = serde_json::to_value(&acknowledgement).unwrap();
    noncanonical["acknowledgement_sha256"] = serde_json::Value::String(d('0'));
    for malformed in [unknown, missing, extra_head, noncanonical] {
        assert_eq!(
            PublicationAcknowledgementIdentity::from_canonical_json(
                &serde_json::to_vec(&malformed).unwrap()
            )
            .unwrap_err()
            .id(),
            SupportedHostLifecycleErrorId::RecoveryUnsafe
        );
    }
    let canonical_value = serde_json::from_slice::<serde_json::Value>(&canonical).unwrap();
    let canonical_object = canonical_value.as_object().unwrap();
    let mut reordered_object = serde_json::Map::new();
    for key in [
        "acknowledgement_sha256",
        "ledger_head",
        "publication_identity_sha256",
        "effect_identity_sha256",
        "schema_version",
    ] {
        reordered_object.insert(key.to_owned(), canonical_object[key].clone());
    }
    let reordered = serde_json::to_vec(&serde_json::Value::Object(reordered_object)).unwrap();
    assert_ne!(reordered, canonical);
    let mut whitespace = b" ".to_vec();
    whitespace.extend_from_slice(&canonical);
    whitespace.push(b'\n');
    for noncanonical_encoding in [reordered, whitespace] {
        assert_eq!(
            PublicationAcknowledgementIdentity::from_canonical_json(&noncanonical_encoding)
                .unwrap_err()
                .id(),
            SupportedHostLifecycleErrorId::RecoveryUnsafe
        );
    }
    let duplicate = String::from_utf8(canonical).unwrap().replacen(
        '{',
        "{\"schema_version\":\"PublicationAcknowledgementIdentity-v1\",",
        1,
    );
    assert_eq!(
        PublicationAcknowledgementIdentity::from_canonical_json(duplicate.as_bytes())
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::RecoveryUnsafe
    );
}

#[test]
fn special_multiple_and_generation_races_fail_before_recovery_acceptance() {
    let expectation =
        publication_expectation(expected_regular("ledger.json", 100, 0o100400, d('1'), true));
    let head = recovery_head(7, 'a');
    let target_prior = regular_with_mode("ledger.json", 10, 100, 0o100400, Some(d('1')), true);
    for (kind, mode) in [
        (PublicationObjectKind::Symlink, 0o120777),
        (PublicationObjectKind::Directory, 0o040700),
        (PublicationObjectKind::Fifo, 0o010600),
        (PublicationObjectKind::Socket, 0o140600),
        (PublicationObjectKind::Device, 0o020600),
        (PublicationObjectKind::Unknown, 0),
    ] {
        let special = PublicationObjectObservation::new(
            ".ledger.json.0123456789abcdef.tmp".to_owned(),
            kind,
            0,
            mode,
            1,
            None,
            10,
            false,
        )
        .unwrap();
        assert_eq!(
            inventory(
                target_prior.clone(),
                vec![special],
                expectation.clone(),
                head.clone(),
                None,
            )
            .classify()
            .unwrap()
            .id(),
            PublicationClassificationId::UnsafeSpecialObject
        );
    }

    let first = regular_with_mode(
        ".ledger.json.0123456789abcdef.tmp",
        10,
        200,
        0o100400,
        Some(d('2')),
        true,
    );
    let second = regular_with_mode(
        ".ledger.json.fedcba9876543210.tmp",
        10,
        200,
        0o100400,
        Some(d('2')),
        true,
    );
    assert_eq!(
        inventory_at(
            10,
            10,
            target_prior.clone(),
            vec![first.clone(), second],
            expectation.clone(),
            head.clone(),
            None,
        )
        .classify()
        .unwrap()
        .id(),
        PublicationClassificationId::MultipleTemporaryObjects
    );
    for race in [
        inventory_at(
            10,
            11,
            target_prior.clone(),
            vec![],
            expectation.clone(),
            head.clone(),
            None,
        ),
        inventory_at(
            10,
            10,
            regular_with_mode("ledger.json", 11, 100, 0o100400, Some(d('1')), true),
            vec![],
            expectation.clone(),
            head.clone(),
            None,
        ),
        inventory_at(
            10,
            10,
            target_prior,
            vec![regular_with_mode(
                ".ledger.json.0123456789abcdef.tmp",
                11,
                200,
                0o100400,
                Some(d('2')),
                true,
            )],
            expectation,
            head,
            None,
        ),
    ] {
        assert_eq!(
            race.classify().unwrap().id(),
            PublicationClassificationId::ObservationRace
        );
    }
}

#[test]
fn recovery_is_proposal_only_and_requires_separate_current_authorization() {
    let fixture = Fixture::new('a');
    let ledger = RecordingLedger::new(0, d('a'));
    let coordinator = SupportedHostLifecycleCoordinator::bind(
        "root-lifecycle-authority".to_owned(),
        "host-effect-ledger".to_owned(),
        &ledger,
    )
    .unwrap();
    let expectation =
        publication_expectation(expected_regular("ledger.json", 100, 0o100400, d('1'), true));
    let classification = inventory(
        regular_with_mode("ledger.json", 10, 100, 0o100400, Some(d('1')), true),
        vec![regular_with_mode(
            ".ledger.json.0123456789abcdef.tmp",
            10,
            200,
            0o100400,
            Some(d('2')),
            true,
        )],
        expectation,
        recovery_head(0, 'a'),
        None,
    )
    .classify()
    .unwrap();
    let mut issue_clock = ProbeClock {
        samples: VecDeque::from([TrustedTimeSample::new(
            "root-monotonic-clock".to_owned(),
            1,
            1,
            90_000,
        )
        .unwrap()]),
        calls: 0,
    };
    let authorization = coordinator
        .authorize_recovery(&classification, &mut issue_clock)
        .unwrap();
    let ledger_before = ledger.writes();
    let fixture_before = recursive_snapshot(&fixture.root);
    let mut proposal_clock = ProbeClock {
        samples: VecDeque::from([TrustedTimeSample::new(
            "root-monotonic-clock".to_owned(),
            1,
            2,
            90_001,
        )
        .unwrap()]),
        calls: 0,
    };
    let proposal = coordinator
        .recovery_proposal(&classification, authorization, &mut proposal_clock)
        .unwrap();
    assert_eq!(
        proposal.action(),
        RecoveryProposalAction::QuarantineTemporaryObjectForReview
    );
    assert!(!proposal.automatic_cleanup());
    assert_eq!(ledger.writes(), ledger_before);
    assert_eq!(recursive_snapshot(&fixture.root), fixture_before);
}

#[test]
fn recovery_authorization_rejects_generation_only_ledger_head_substitution() {
    let ledger = RecordingLedger::new(0, d('a'));
    let coordinator = SupportedHostLifecycleCoordinator::bind(
        "root-lifecycle-authority".to_owned(),
        "host-effect-ledger".to_owned(),
        &ledger,
    )
    .unwrap();
    let expectation =
        publication_expectation(expected_regular("ledger.json", 100, 0o100400, d('1'), true));
    let classification = inventory(
        regular_with_mode("ledger.json", 10, 100, 0o100400, Some(d('1')), true),
        vec![regular_with_mode(
            ".ledger.json.0123456789abcdef.tmp",
            10,
            200,
            0o100400,
            Some(d('2')),
            true,
        )],
        expectation,
        recovery_head(0, 'a'),
        None,
    )
    .classify()
    .unwrap();

    let mut issue_clock = one_sample_clock(95_000, 1);
    let authorization = coordinator
        .authorize_recovery(&classification, &mut issue_clock)
        .unwrap();
    ledger.inner.lock().unwrap().head = HostEffectLedgerHead::new(1, d('a')).unwrap();
    let mut proposal_clock = one_sample_clock(95_001, 2);
    assert_eq!(
        coordinator
            .recovery_proposal(&classification, authorization, &mut proposal_clock)
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::RecoveryAuthorizationRequired
    );
    assert_eq!(ledger.writes(), 0);
}

#[test]
fn recovery_authorization_rejects_classification_head_and_expiry_substitution() {
    let ledger = RecordingLedger::new(0, d('a'));
    let coordinator = SupportedHostLifecycleCoordinator::bind(
        "root-lifecycle-authority".to_owned(),
        "host-effect-ledger".to_owned(),
        &ledger,
    )
    .unwrap();
    let expectation =
        publication_expectation(expected_regular("ledger.json", 100, 0o100400, d('1'), true));
    let before_rename = inventory(
        regular_with_mode("ledger.json", 10, 100, 0o100400, Some(d('1')), true),
        vec![regular_with_mode(
            ".ledger.json.0123456789abcdef.tmp",
            10,
            200,
            0o100400,
            Some(d('2')),
            true,
        )],
        expectation.clone(),
        recovery_head(0, 'a'),
        None,
    )
    .classify()
    .unwrap();
    let committed = inventory(
        regular_with_mode("ledger.json", 10, 200, 0o100400, Some(d('2')), true),
        vec![],
        expectation,
        recovery_head(0, 'a'),
        None,
    )
    .classify()
    .unwrap();

    let mut issue_clock = one_sample_clock(100_000, 1);
    let authorization = coordinator
        .authorize_recovery(&before_rename, &mut issue_clock)
        .unwrap();
    let mut proposal_clock = one_sample_clock(100_001, 2);
    assert_eq!(
        coordinator
            .recovery_proposal(&committed, authorization, &mut proposal_clock)
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::RecoveryAuthorizationRequired
    );

    let mut issue_clock = one_sample_clock(110_000, 3);
    let authorization = coordinator
        .authorize_recovery(&before_rename, &mut issue_clock)
        .unwrap();
    ledger.inner.lock().unwrap().head = HostEffectLedgerHead::new(0, d('b')).unwrap();
    let mut proposal_clock = one_sample_clock(110_001, 4);
    assert_eq!(
        coordinator
            .recovery_proposal(&before_rename, authorization, &mut proposal_clock)
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::RecoveryAuthorizationRequired
    );

    ledger.inner.lock().unwrap().head = HostEffectLedgerHead::new(0, d('a')).unwrap();
    let mut issue_clock = one_sample_clock(115_000, 5);
    let authorization = coordinator
        .authorize_recovery(&before_rename, &mut issue_clock)
        .unwrap();
    ledger.inner.lock().unwrap().head = HostEffectLedgerHead::new(1, d('b')).unwrap();
    let mut proposal_clock = one_sample_clock(115_001, 6);
    assert_eq!(
        coordinator
            .recovery_proposal(&before_rename, authorization, &mut proposal_clock)
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::RecoveryAuthorizationRequired
    );

    ledger.inner.lock().unwrap().head = HostEffectLedgerHead::new(0, d('a')).unwrap();
    let mut issue_clock = one_sample_clock(120_000, 7);
    let authorization = coordinator
        .authorize_recovery(&before_rename, &mut issue_clock)
        .unwrap();
    let mut expired_clock = one_sample_clock(180_001, 8);
    assert_eq!(
        coordinator
            .recovery_proposal(&before_rename, authorization, &mut expired_clock)
            .unwrap_err()
            .id(),
        SupportedHostLifecycleErrorId::RecoveryAuthorizationRequired
    );
    assert_eq!(ledger.writes(), 0);
}

fn one_sample_clock(unix_ms: u64, sequence: u64) -> ProbeClock {
    ProbeClock {
        samples: VecDeque::from([TrustedTimeSample::new(
            "root-monotonic-clock".to_owned(),
            1,
            sequence,
            unix_ms,
        )
        .unwrap()]),
        calls: 0,
    }
}

fn regular_with_mode(
    name: &str,
    generation: u64,
    byte_length: u64,
    mode: u32,
    content_sha256: Option<String>,
    data_synced: bool,
) -> PublicationObjectObservation {
    PublicationObjectObservation::new(
        name.to_owned(),
        PublicationObjectKind::Regular,
        byte_length,
        mode,
        1,
        content_sha256,
        generation,
        data_synced,
    )
    .unwrap()
}

fn expected_regular(
    name: &str,
    byte_length: u64,
    mode: u32,
    content_sha256: String,
    data_synced: bool,
) -> ExpectedPublicationObjectIdentity {
    ExpectedPublicationObjectIdentity::regular(
        name.to_owned(),
        byte_length,
        mode,
        1,
        content_sha256,
        data_synced,
    )
    .unwrap()
}

fn expected_missing(name: &str) -> ExpectedPublicationObjectIdentity {
    ExpectedPublicationObjectIdentity::missing(name.to_owned()).unwrap()
}

fn publication_expectation(prior: ExpectedPublicationObjectIdentity) -> PublicationExpectation {
    publication_expectation_with_effect(d('e'), prior)
}

fn publication_expectation_with_effect(
    effect_identity_sha256: String,
    prior: ExpectedPublicationObjectIdentity,
) -> PublicationExpectation {
    publication_expectation_exact(
        effect_identity_sha256,
        prior,
        "ledger.json",
        ".ledger.json.0123456789abcdef.tmp",
        200,
        0o100400,
        d('2'),
        true,
    )
    .unwrap()
}

#[allow(clippy::too_many_arguments)]
fn publication_expectation_exact(
    effect_identity_sha256: String,
    prior: ExpectedPublicationObjectIdentity,
    target_name: &str,
    temporary_name: &str,
    byte_length: u64,
    mode: u32,
    content_sha256: String,
    data_synced: bool,
) -> Result<PublicationExpectation, SupportedHostLifecycleError> {
    PublicationExpectation::new(
        effect_identity_sha256,
        prior,
        ExpectedPublicationObjectIdentity::regular(
            target_name.to_owned(),
            byte_length,
            mode,
            1,
            content_sha256.clone(),
            data_synced,
        )?,
        ExpectedPublicationObjectIdentity::regular(
            temporary_name.to_owned(),
            byte_length,
            mode,
            1,
            content_sha256,
            data_synced,
        )?,
    )
}

fn recovery_head(generation: u64, digest_byte: char) -> HostEffectLedgerHead {
    HostEffectLedgerHead::new(generation, d(digest_byte)).unwrap()
}

fn inventory(
    target: PublicationObjectObservation,
    temporary_objects: Vec<PublicationObjectObservation>,
    expectation: PublicationExpectation,
    current_ledger_head: HostEffectLedgerHead,
    acknowledgement: Option<PublicationAcknowledgementIdentity>,
) -> PublicationInventoryObservation {
    inventory_at(
        10,
        10,
        target,
        temporary_objects,
        expectation,
        current_ledger_head,
        acknowledgement,
    )
}

#[allow(clippy::too_many_arguments)]
fn inventory_at(
    scan_generation_before: u64,
    scan_generation_after: u64,
    target: PublicationObjectObservation,
    temporary_objects: Vec<PublicationObjectObservation>,
    expectation: PublicationExpectation,
    current_ledger_head: HostEffectLedgerHead,
    acknowledgement: Option<PublicationAcknowledgementIdentity>,
) -> PublicationInventoryObservation {
    PublicationInventoryObservation::new(
        scan_generation_before,
        scan_generation_after,
        target,
        temporary_objects,
        expectation,
        current_ledger_head,
        acknowledgement,
    )
    .unwrap()
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SnapshotRow {
    relative: String,
    kind: &'static str,
    mode: u32,
    byte_length: u64,
    content_sha256: Option<String>,
}

fn recursive_snapshot(root: &Path) -> Vec<SnapshotRow> {
    let mut rows = Vec::new();
    snapshot_directory(root, root, &mut rows);
    rows.sort_by(|left, right| left.relative.cmp(&right.relative));
    rows
}

fn snapshot_directory(root: &Path, current: &Path, rows: &mut Vec<SnapshotRow>) {
    let mut entries = fs::read_dir(current)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).unwrap();
        let relative = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        #[cfg(unix)]
        let mode = metadata.mode();
        #[cfg(not(unix))]
        let mode = 0;
        if metadata.is_dir() {
            rows.push(SnapshotRow {
                relative,
                kind: "directory",
                mode,
                byte_length: 0,
                content_sha256: None,
            });
            snapshot_directory(root, &path, rows);
        } else if metadata.is_file() {
            let bytes = fs::read(&path).unwrap();
            rows.push(SnapshotRow {
                relative,
                kind: "file",
                mode,
                byte_length: bytes.len() as u64,
                content_sha256: Some(digest(&bytes)),
            });
        } else {
            rows.push(SnapshotRow {
                relative,
                kind: "special",
                mode,
                byte_length: metadata.len(),
                content_sha256: None,
            });
        }
    }
}
