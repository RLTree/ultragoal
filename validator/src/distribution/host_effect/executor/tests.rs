use super::*;
use crate::distribution::host_effect::executor::model::CommandCapture;
use crate::distribution::host_effect::executor::process::BackendFailure;
use crate::distribution::host_effect::executor::target::{FaultPoint, set_fault};
use crate::distribution::host_effect::lifecycle::{
    AcceptedHostScope, DescriptorExecutionPrimitive, HostTargetObserver,
    PublicationAcknowledgementIdentity, PublicationClassificationId, SupportedHostLifecycleErrorId,
    TrustedTimeSample, lifecycle_error,
};
use crate::distribution::host_effect::{
    DurableHostEffectLedger, FileHostEffectLedger, HostEffectAuthority, HostEffectDecision,
    HostEffectLedgerError, HostEffectLedgerErrorId, HostEffectLedgerHead, HostEffectLedgerRecord,
    HostEffectPermitBinding, HostEffectReservation, HostEffectState, HostEffectTransition,
    PinnedHostExecutable,
};
use crate::distribution::{
    HostCapabilityDeclaration, HostCommandPlan, JourneyBinding, PackageIdentity, SourceIdentity,
};
use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    roots: Vec<PathBuf>,
    target_root: PathBuf,
    ledger_root: PathBuf,
    executable: PathBuf,
    home: PathBuf,
    project: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let suffix = format!(
            "{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        );
        let target_root = PathBuf::from(format!("/private/tmp/hul-supported-host-effect-{suffix}"));
        let ledger_root = PathBuf::from(format!("/private/tmp/hul-executor-ledger-{suffix}"));
        let support_root = PathBuf::from(format!("/private/tmp/hul-executor-support-{suffix}"));
        create_mode(&target_root, 0o700);
        create_mode(&support_root, 0o700);
        let home = support_root.join("home");
        let project = support_root.join("project");
        create_mode(&home, 0o700);
        create_mode(&project, 0o700);
        let executable = support_root.join("codex-fixture");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&executable)
            .unwrap();
        file.write_all(b"#!/bin/sh\nexit 0\n").unwrap();
        file.sync_all().unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        Self {
            roots: vec![target_root.clone(), ledger_root.clone(), support_root],
            target_root,
            ledger_root,
            executable,
            home,
            project,
        }
    }

    fn package(&self) -> PackageIdentity {
        let source = SourceIdentity::new(
            digest('1'),
            digest('2'),
            "harness-ultragoal".to_owned(),
            "0.0.11".to_owned(),
            digest('3'),
            digest('4'),
        )
        .unwrap();
        PackageIdentity::new(source, digest('5'), digest('6')).unwrap()
    }

    fn scope_and_target(
        &self,
    ) -> (
        AcceptedHostScope,
        ConfinedHostEffectTarget,
        crate::distribution::host_effect::lifecycle::ObservedTargetIdentity,
    ) {
        let package = self.package();
        let host = HostCapabilityDeclaration::isolated(
            &self.home,
            &self.project,
            "fixture-host",
            Some(&self.executable),
        )
        .unwrap();
        let journey = JourneyBinding::new(package, &host, "fixture-marketplace").unwrap();
        let scope =
            AcceptedHostScope::personal(&journey, "fixture-marketplace".to_owned()).unwrap();
        let (target, identity) =
            ConfinedHostEffectTarget::bind(&self.target_root, scope.clone(), 1).unwrap();
        (scope, target, identity)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for root in self.roots.iter().rev() {
            if root.starts_with("/private/tmp/hul-") {
                let _ = fs::remove_dir_all(root);
            }
        }
    }
}

fn create_mode(path: &Path, mode: u32) {
    fs::DirBuilder::new().mode(mode).create(path).unwrap();
}

fn digest(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn permit_binding(
    fixture: &Fixture,
    ledger: &FileHostEffectLedger,
    target: &crate::distribution::host_effect::lifecycle::ObservedTargetIdentity,
    candidate_digest_byte: char,
) -> HostEffectPermitBinding {
    let package = fixture.package();
    let plan = HostCommandPlan::personal_install(&package, "fixture-marketplace").unwrap();
    let executable = PinnedHostExecutable::pin(&fixture.executable).unwrap();
    let head = ledger.head().unwrap();
    HostEffectPermitBinding {
        context_id: package.source().context_id().to_owned(),
        candidate_id: digest(candidate_digest_byte),
        package_identity_sha256: digest('3'),
        journey_binding_sha256: digest('4'),
        session_issuance_sha256: digest('5'),
        lifecycle_plan_sha256: digest('6'),
        lifecycle_intent: "install".to_owned(),
        expected_pre_state_sha256: digest('7'),
        expected_post_state_sha256: digest('8'),
        rollback_policy_sha256: digest('9'),
        reconciliation_policy_sha256: digest('a'),
        host_scope_sha256: digest('b'),
        host_capability_sha256: digest('c'),
        required_capabilities_sha256: digest('d'),
        external_request_sha256: digest('e'),
        command_plan_sha256: plan.plan_sha256().to_owned(),
        argv_sha256: digest('f'),
        executable_identity_sha256: executable.identity().binding_sha256().unwrap(),
        target_identity_sha256: target.target_sha256().to_owned(),
        target_generation: target.generation(),
        issued_at_unix_ms: 1_000,
        expires_at_unix_ms: 200_000,
        expected_head_sha256: head.head_sha256().to_owned(),
        decision: HostEffectDecision::Authorize,
    }
}

fn authorized_effect(
    fixture: &Fixture,
    ledger: &FileHostEffectLedger,
    target: &crate::distribution::host_effect::lifecycle::ObservedTargetIdentity,
) -> AuthorizedHostEffect {
    let package = fixture.package();
    let plan = HostCommandPlan::personal_install(&package, "fixture-marketplace").unwrap();
    let executable = PinnedHostExecutable::pin(&fixture.executable).unwrap();
    let binding = permit_binding(fixture, ledger, target, '2');
    let authority =
        HostEffectAuthority::generate("fixture-root".to_owned(), "fixture-ledger".to_owned())
            .unwrap();
    let (permit, reservation) = authority.issue(binding).unwrap();
    let reserved = ledger.reserve(reservation).unwrap();
    let in_flight = ledger
        .transition(
            HostEffectTransition::new(
                permit.permit_id().to_owned(),
                HostEffectState::Reserved,
                HostEffectState::InFlight,
                reserved.current_head().clone(),
                None,
            )
            .unwrap(),
        )
        .unwrap();
    AuthorizedHostEffect::new(permit, in_flight, executable, plan).unwrap()
}

struct TestClock {
    sequence: u64,
}

impl RootTrustedClock for TestClock {
    fn sample(
        &mut self,
    ) -> Result<
        TrustedTimeSample,
        crate::distribution::host_effect::lifecycle::SupportedHostLifecycleError,
    > {
        self.sequence += 1;
        TrustedTimeSample::new(
            "executor-test-clock".to_owned(),
            1,
            self.sequence,
            10_000 + self.sequence,
        )
    }
}

struct FailingClock;

impl RootTrustedClock for FailingClock {
    fn sample(
        &mut self,
    ) -> Result<
        TrustedTimeSample,
        crate::distribution::host_effect::lifecycle::SupportedHostLifecycleError,
    > {
        Err(lifecycle_error(
            SupportedHostLifecycleErrorId::UntrustedTime,
        ))
    }
}

enum BackendReply {
    Success(Vec<u8>, Vec<u8>),
    Failure(HostEffectExecutorErrorId, bool),
    Oversized,
}

struct ScriptedBackend {
    replies: VecDeque<BackendReply>,
    calls: usize,
}

impl ScriptedBackend {
    fn success() -> Self {
        Self {
            replies: VecDeque::from([BackendReply::Success(b"ok\n".to_vec(), Vec::new())]),
            calls: 0,
        }
    }
}

impl RetainedDescriptorProcessBackend for ScriptedBackend {
    fn execute(
        &mut self,
        _capability: &DescriptorExecutionCapability,
        _executable: &PinnedHostExecutable,
        _command: &crate::distribution::HostCommand,
        _policy: &HostEffectExecutionPolicy,
        _cancellation: &HostEffectCancellation,
    ) -> Result<CommandCapture, BackendFailure> {
        self.calls += 1;
        match self.replies.pop_front().unwrap() {
            BackendReply::Success(stdout, stderr) => Ok(CommandCapture {
                exit_code: 0,
                stdout,
                stderr,
            }),
            BackendReply::Failure(id, started) => Err(BackendFailure {
                id,
                started,
                capture: CommandCapture {
                    exit_code: -1,
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                },
            }),
            BackendReply::Oversized => Ok(CommandCapture {
                exit_code: 0,
                stdout: vec![b'x'; model::EXACT_OUTPUT_LIMIT_BYTES + 1],
                stderr: Vec::new(),
            }),
        }
    }
}

fn capability() -> DescriptorExecutionCapability {
    DescriptorExecutionCapability::new(
        DescriptorExecutionPlatform::Linux,
        DescriptorExecutionPrimitive::ExecveAtEmptyPath,
        "fixture-execveat".to_owned(),
        "v1".to_owned(),
    )
    .unwrap()
}

struct FailTerminalLedger<'a> {
    inner: &'a FileHostEffectLedger,
}

impl DurableHostEffectLedger for FailTerminalLedger<'_> {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError> {
        self.inner.head()
    }

    fn reserve(
        &self,
        reservation: HostEffectReservation,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        self.inner.reserve(reservation)
    }

    fn transition(
        &self,
        _transition: HostEffectTransition,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
    }

    fn read(
        &self,
        permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError> {
        self.inner.read(permit_id)
    }
}

struct DisplaceTargetAndFailTerminalLedger<'a> {
    inner: &'a FileHostEffectLedger,
    target_root: PathBuf,
    displaced_root: PathBuf,
    transition_calls: Arc<AtomicU64>,
}

struct CommitThenDisplaceTargetLedger<'a> {
    inner: &'a FileHostEffectLedger,
    target_root: PathBuf,
    displaced_root: PathBuf,
    transition_calls: Arc<AtomicU64>,
}

impl DurableHostEffectLedger for CommitThenDisplaceTargetLedger<'_> {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError> {
        self.inner.head()
    }

    fn reserve(
        &self,
        reservation: HostEffectReservation,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        self.inner.reserve(reservation)
    }

    fn transition(
        &self,
        transition: HostEffectTransition,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        self.transition_calls.fetch_add(1, Ordering::SeqCst);
        let terminal = self.inner.transition(transition)?;
        fs::rename(&self.target_root, &self.displaced_root)
            .map_err(|_| HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))?;
        Ok(terminal)
    }

    fn read(
        &self,
        permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError> {
        self.inner.read(permit_id)
    }
}

impl DurableHostEffectLedger for DisplaceTargetAndFailTerminalLedger<'_> {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError> {
        self.inner.head()
    }

    fn reserve(
        &self,
        reservation: HostEffectReservation,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        self.inner.reserve(reservation)
    }

    fn transition(
        &self,
        _transition: HostEffectTransition,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        self.transition_calls.fetch_add(1, Ordering::SeqCst);
        fs::rename(&self.target_root, &self.displaced_root)
            .map_err(|_| HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))?;
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
    }

    fn read(
        &self,
        permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError> {
        self.inner.read(permit_id)
    }
}

struct CommitThenFailTerminalLedger<'a> {
    inner: &'a FileHostEffectLedger,
}

impl DurableHostEffectLedger for CommitThenFailTerminalLedger<'_> {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError> {
        self.inner.head()
    }

    fn reserve(
        &self,
        reservation: HostEffectReservation,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        self.inner.reserve(reservation)
    }

    fn transition(
        &self,
        transition: HostEffectTransition,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        self.inner.transition(transition)?;
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
    }

    fn read(
        &self,
        permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError> {
        self.inner.read(permit_id)
    }
}

struct MutateCommitThenFailTerminalLedger<'a> {
    inner: &'a FileHostEffectLedger,
}

impl DurableHostEffectLedger for MutateCommitThenFailTerminalLedger<'_> {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError> {
        self.inner.head()
    }

    fn reserve(
        &self,
        reservation: HostEffectReservation,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        self.inner.reserve(reservation)
    }

    fn transition(
        &self,
        mut transition: HostEffectTransition,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        transition.next_state = HostEffectState::Failed;
        self.inner.transition(transition)?;
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
    }

    fn read(
        &self,
        permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError> {
        self.inner.read(permit_id)
    }
}

#[derive(Clone, Copy)]
enum RecoveryObservationMode {
    Current,
    Error,
    SubstituteUnrelatedRecord,
    StaleHead,
}

struct AdvanceUnrelatedPermitThenFailTerminalLedger<'a> {
    inner: &'a FileHostEffectLedger,
    authority: HostEffectAuthority,
    unrelated_binding: HostEffectPermitBinding,
    stale_head: HostEffectLedgerHead,
    commit_terminal: bool,
    repeat_advance: bool,
    observation_mode: RecoveryObservationMode,
    advanced: AtomicBool,
    unrelated_record: Mutex<Option<HostEffectLedgerRecord>>,
}

impl AdvanceUnrelatedPermitThenFailTerminalLedger<'_> {
    fn advance_unrelated_permit(&self) -> Result<(), HostEffectLedgerError> {
        let mut binding = self.unrelated_binding.clone();
        binding.expected_head_sha256 = self.inner.head()?.head_sha256().to_owned();
        let (_permit, reservation) = self
            .authority
            .issue(binding)
            .map_err(|_| HostEffectLedgerError::new(HostEffectLedgerErrorId::InvalidRecord))?;
        let reserved = self.inner.reserve(reservation)?;
        let latest = if self.repeat_advance {
            self.inner.transition(
                HostEffectTransition::new(
                    reserved.reservation().permit_id().to_owned(),
                    HostEffectState::Reserved,
                    HostEffectState::InFlight,
                    reserved.current_head().clone(),
                    None,
                )
                .unwrap(),
            )?
        } else {
            reserved
        };
        *self
            .unrelated_record
            .lock()
            .map_err(|_| HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))? = Some(latest);
        self.advanced.store(true, Ordering::SeqCst);
        Ok(())
    }
}

impl DurableHostEffectLedger for AdvanceUnrelatedPermitThenFailTerminalLedger<'_> {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError> {
        if self.advanced.load(Ordering::SeqCst) {
            match self.observation_mode {
                RecoveryObservationMode::Error => {
                    return Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io));
                }
                RecoveryObservationMode::StaleHead => return Ok(self.stale_head.clone()),
                RecoveryObservationMode::Current
                | RecoveryObservationMode::SubstituteUnrelatedRecord => {}
            }
        }
        self.inner.head()
    }

    fn reserve(
        &self,
        reservation: HostEffectReservation,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        self.inner.reserve(reservation)
    }

    fn transition(
        &self,
        transition: HostEffectTransition,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        if self.commit_terminal {
            self.inner.transition(transition)?;
        }
        self.advance_unrelated_permit()?;
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
    }

    fn read(
        &self,
        permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError> {
        if self.advanced.load(Ordering::SeqCst) {
            match self.observation_mode {
                RecoveryObservationMode::Error => {
                    return Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io));
                }
                RecoveryObservationMode::SubstituteUnrelatedRecord => {
                    return self
                        .unrelated_record
                        .lock()
                        .map_err(|_| HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
                        .map(|record| record.clone());
                }
                RecoveryObservationMode::Current | RecoveryObservationMode::StaleHead => {}
            }
        }
        self.inner.read(permit_id)
    }
}

struct UnavailableObservationLedger;

impl DurableHostEffectLedger for UnavailableObservationLedger {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError> {
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
    }

    fn reserve(
        &self,
        _reservation: HostEffectReservation,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
    }

    fn transition(
        &self,
        _transition: HostEffectTransition,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
    }

    fn read(
        &self,
        _permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError> {
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
    }
}

#[derive(Clone, Copy)]
enum PostReservationObservationMode {
    Stable,
    ReadError,
}

struct PostReservationObservationLedger {
    expected_permit_id: String,
    observed_head: HostEffectLedgerHead,
    observed_record: HostEffectLedgerRecord,
    mode: PostReservationObservationMode,
    head_calls: AtomicU64,
    read_calls: AtomicU64,
}

impl PostReservationObservationLedger {
    fn stable(
        expected_permit_id: String,
        observed_head: HostEffectLedgerHead,
        observed_record: HostEffectLedgerRecord,
    ) -> Self {
        Self {
            expected_permit_id,
            observed_head,
            observed_record,
            mode: PostReservationObservationMode::Stable,
            head_calls: AtomicU64::new(0),
            read_calls: AtomicU64::new(0),
        }
    }

    fn read_error(
        expected_permit_id: String,
        observed_head: HostEffectLedgerHead,
        observed_record: HostEffectLedgerRecord,
    ) -> Self {
        Self {
            expected_permit_id,
            observed_head,
            observed_record,
            mode: PostReservationObservationMode::ReadError,
            head_calls: AtomicU64::new(0),
            read_calls: AtomicU64::new(0),
        }
    }
}

impl DurableHostEffectLedger for PostReservationObservationLedger {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError> {
        self.head_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.observed_head.clone())
    }

    fn reserve(
        &self,
        _reservation: HostEffectReservation,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
    }

    fn transition(
        &self,
        _transition: HostEffectTransition,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
    }

    fn read(
        &self,
        permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError> {
        self.read_calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(permit_id, self.expected_permit_id);
        match self.mode {
            PostReservationObservationMode::Stable => Ok(Some(self.observed_record.clone())),
            PostReservationObservationMode::ReadError => {
                Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
            }
        }
    }
}

struct ReplayThenWrongPermitObservationLedger {
    expected_permit_id: String,
    replay_record: HostEffectLedgerRecord,
    observed_head: HostEffectLedgerHead,
    wrong_permit_record: HostEffectLedgerRecord,
    head_calls: AtomicU64,
    read_calls: AtomicU64,
}

impl DurableHostEffectLedger for ReplayThenWrongPermitObservationLedger {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError> {
        self.head_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.observed_head.clone())
    }

    fn reserve(
        &self,
        _reservation: HostEffectReservation,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
    }

    fn transition(
        &self,
        _transition: HostEffectTransition,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
    }

    fn read(
        &self,
        permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError> {
        assert_eq!(permit_id, self.expected_permit_id);
        let call = self.read_calls.fetch_add(1, Ordering::SeqCst);
        if call == 0 {
            Ok(Some(self.replay_record.clone()))
        } else {
            Ok(Some(self.wrong_permit_record.clone()))
        }
    }
}

fn unrelated_record(
    fixture: &Fixture,
    ledger: &FileHostEffectLedger,
    target: &crate::distribution::host_effect::lifecycle::ObservedTargetIdentity,
    state: HostEffectState,
) -> HostEffectLedgerRecord {
    let authority =
        HostEffectAuthority::generate("unrelated-root".to_owned(), "fixture-ledger".to_owned())
            .unwrap();
    let (_permit, reservation) = authority
        .issue(permit_binding(fixture, ledger, target, '0'))
        .unwrap();
    let reserved = ledger.reserve(reservation).unwrap();
    if state == HostEffectState::Reserved {
        return reserved;
    }
    let in_flight = ledger
        .transition(
            HostEffectTransition::new(
                reserved.reservation().permit_id().to_owned(),
                HostEffectState::Reserved,
                HostEffectState::InFlight,
                reserved.current_head().clone(),
                None,
            )
            .unwrap(),
        )
        .unwrap();
    if state == HostEffectState::InFlight {
        return in_flight;
    }
    ledger
        .transition(
            HostEffectTransition::new(
                in_flight.reservation().permit_id().to_owned(),
                HostEffectState::InFlight,
                state,
                in_flight.current_head().clone(),
                Some(digest('d')),
            )
            .unwrap(),
        )
        .unwrap()
}

fn recovering_terminal_record(
    ledger: &FileHostEffectLedger,
    effect: &AuthorizedHostEffect,
) -> HostEffectLedgerRecord {
    ledger
        .transition(
            HostEffectTransition::new(
                effect.permit().permit_id().to_owned(),
                HostEffectState::InFlight,
                HostEffectState::Ambiguous,
                effect.record().current_head().clone(),
                Some(digest('d')),
            )
            .unwrap(),
        )
        .unwrap()
}

fn direct_post_reservation_failure(
    target: ConfinedHostEffectTarget,
    ledger: &PostReservationObservationLedger,
    effect: &AuthorizedHostEffect,
    originating_error_ids: Vec<HostEffectExecutorErrorId>,
) -> HostEffectExecutorFailure {
    let mut backend = ScriptedBackend::success();
    let mut executor = SupportedHostEffectExecutor::new(
        ledger,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );
    let effect_identity = executor.effect_identity(&capability(), effect).unwrap();
    let failure = executor.post_reservation_recovery_failure(
        effect,
        &effect_identity,
        HostEffectExecutorErrorId::RecoveryRequired,
        originating_error_ids,
        None,
    );
    drop(executor);
    assert_eq!(backend.calls, 0);
    failure
}

fn assert_bound_unavailable_post_reservation(
    failure: &HostEffectExecutorFailure,
    returned_error_id: HostEffectExecutorErrorId,
    effect: &AuthorizedHostEffect,
    expected_error_ids: &[HostEffectExecutorErrorId],
) {
    assert_eq!(failure.id(), returned_error_id);
    assert_eq!(failure.terminal_state(), None);
    let recovery = failure.recovery().unwrap();
    assert_eq!(recovery.permit_id(), effect.permit().permit_id());
    assert_eq!(recovery.ledger_head(), effect.record().current_head());
    assert_eq!(recovery.ledger_record().unwrap(), effect.record());
    assert!(!recovery.has_exact_current_ledger_observation());
    assert!(!recovery.has_exact_current_publication_observation());
    assert_eq!(
        recovery.post_reservation_ledger_classification(),
        Some(HostEffectPostReservationLedgerClassification::ObservationUnavailable)
    );
    assert_eq!(
        recovery.post_reservation_publication_classification(),
        Some(HostEffectPostReservationPublicationClassification::NoPublicationEvidence)
    );
    assert_eq!(recovery.originating_error_ids(), expected_error_ids);
    assert!(recovery.terminal_classification().is_none());
    assert!(recovery.outcome().is_none());
    assert!(recovery.observation().is_none());
    assert!(recovery.prior_publication_observation().is_none());
    assert!(recovery.publication_identity_sha256().is_none());
    assert!(recovery.classification().is_none());
    assert!(recovery.verify_binding());

    let mut substituted = recovery.clone();
    substituted.substitute_permit_without_rebinding_for_test(digest('0'));
    assert!(!substituted.verify_binding());
}

#[test]
fn wrong_permit_post_reservation_still_in_flight_observation_falls_back_bound_unavailable() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let wrong_record = unrelated_record(
        &fixture,
        &durable,
        &target_identity,
        HostEffectState::InFlight,
    );
    assert_ne!(
        wrong_record.reservation().permit_id(),
        effect.permit().permit_id()
    );
    let ledger = PostReservationObservationLedger::stable(
        effect.permit().permit_id().to_owned(),
        wrong_record.current_head().clone(),
        wrong_record,
    );
    let failure = direct_post_reservation_failure(
        target,
        &ledger,
        &effect,
        vec![
            HostEffectExecutorErrorId::Timeout,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ],
    );
    assert_bound_unavailable_post_reservation(
        &failure,
        HostEffectExecutorErrorId::RecoveryRequired,
        &effect,
        &[
            HostEffectExecutorErrorId::Timeout,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ],
    );
    assert_eq!(ledger.head_calls.load(Ordering::SeqCst), 6);
    assert_eq!(ledger.read_calls.load(Ordering::SeqCst), 6);
}

#[test]
fn wrong_permit_post_reservation_terminal_observation_falls_back_bound_unavailable() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let wrong_record = unrelated_record(
        &fixture,
        &durable,
        &target_identity,
        HostEffectState::Ambiguous,
    );
    assert_ne!(
        wrong_record.reservation().permit_id(),
        effect.permit().permit_id()
    );
    let ledger = PostReservationObservationLedger::stable(
        effect.permit().permit_id().to_owned(),
        wrong_record.current_head().clone(),
        wrong_record,
    );
    let failure = direct_post_reservation_failure(
        target,
        &ledger,
        &effect,
        vec![
            HostEffectExecutorErrorId::Timeout,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ],
    );
    assert_bound_unavailable_post_reservation(
        &failure,
        HostEffectExecutorErrorId::RecoveryRequired,
        &effect,
        &[
            HostEffectExecutorErrorId::Timeout,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ],
    );
    assert_eq!(ledger.head_calls.load(Ordering::SeqCst), 6);
    assert_eq!(ledger.read_calls.load(Ordering::SeqCst), 6);
}

#[test]
fn wrong_permit_post_reservation_rejected_observation_falls_back_bound_unavailable() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let wrong_record = unrelated_record(
        &fixture,
        &durable,
        &target_identity,
        HostEffectState::Reserved,
    );
    assert_ne!(
        wrong_record.reservation().permit_id(),
        effect.permit().permit_id()
    );
    let ledger = PostReservationObservationLedger::stable(
        effect.permit().permit_id().to_owned(),
        wrong_record.current_head().clone(),
        wrong_record,
    );
    let failure = direct_post_reservation_failure(
        target,
        &ledger,
        &effect,
        vec![
            HostEffectExecutorErrorId::Timeout,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ],
    );
    assert_bound_unavailable_post_reservation(
        &failure,
        HostEffectExecutorErrorId::RecoveryRequired,
        &effect,
        &[
            HostEffectExecutorErrorId::Timeout,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ],
    );
    assert_eq!(ledger.head_calls.load(Ordering::SeqCst), 6);
    assert_eq!(ledger.read_calls.load(Ordering::SeqCst), 6);
}

#[test]
fn post_reservation_right_record_wrong_head_falls_back_bound_unavailable() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let unrelated = unrelated_record(
        &fixture,
        &durable,
        &target_identity,
        HostEffectState::Reserved,
    );
    assert_ne!(unrelated.current_head(), effect.record().current_head());
    let ledger = PostReservationObservationLedger::stable(
        effect.permit().permit_id().to_owned(),
        unrelated.current_head().clone(),
        effect.record().clone(),
    );
    let failure = direct_post_reservation_failure(
        target,
        &ledger,
        &effect,
        vec![
            HostEffectExecutorErrorId::Timeout,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ],
    );
    assert_bound_unavailable_post_reservation(
        &failure,
        HostEffectExecutorErrorId::RecoveryRequired,
        &effect,
        &[
            HostEffectExecutorErrorId::Timeout,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ],
    );
    assert_eq!(ledger.head_calls.load(Ordering::SeqCst), 6);
    assert_eq!(ledger.read_calls.load(Ordering::SeqCst), 6);
}

#[test]
fn post_reservation_correct_permit_still_in_flight_positive_control_is_exact_and_bound() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let ledger = PostReservationObservationLedger::stable(
        effect.permit().permit_id().to_owned(),
        effect.record().current_head().clone(),
        effect.record().clone(),
    );
    let failure = direct_post_reservation_failure(
        target,
        &ledger,
        &effect,
        vec![HostEffectExecutorErrorId::Timeout],
    );
    assert_eq!(failure.terminal_state(), None);
    let recovery = failure.recovery().unwrap();
    assert!(recovery.has_exact_current_ledger_observation());
    assert_eq!(
        recovery.post_reservation_ledger_classification(),
        Some(HostEffectPostReservationLedgerClassification::StillInFlight)
    );
    assert_eq!(
        recovery.originating_error_ids(),
        &[HostEffectExecutorErrorId::Timeout]
    );
    assert!(recovery.outcome().is_none());
    assert!(recovery.observation().is_none());
    assert!(recovery.verify_binding());
    assert_eq!(ledger.head_calls.load(Ordering::SeqCst), 2);
    assert_eq!(ledger.read_calls.load(Ordering::SeqCst), 2);
}

#[test]
fn post_reservation_correct_permit_terminal_positive_control_is_exact_and_bound() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let terminal = recovering_terminal_record(&durable, &effect);
    let ledger = PostReservationObservationLedger::stable(
        effect.permit().permit_id().to_owned(),
        terminal.current_head().clone(),
        terminal.clone(),
    );
    let failure = direct_post_reservation_failure(
        target,
        &ledger,
        &effect,
        vec![HostEffectExecutorErrorId::Timeout],
    );
    assert_eq!(failure.terminal_state(), Some(HostEffectState::Ambiguous));
    let recovery = failure.recovery().unwrap();
    assert!(recovery.has_exact_current_ledger_observation());
    assert_eq!(recovery.ledger_record().unwrap(), &terminal);
    assert_eq!(
        recovery.post_reservation_ledger_classification(),
        Some(HostEffectPostReservationLedgerClassification::TerminalObserved)
    );
    assert_eq!(
        recovery.originating_error_ids(),
        &[HostEffectExecutorErrorId::Timeout]
    );
    assert!(recovery.outcome().is_none());
    assert!(recovery.observation().is_none());
    assert!(recovery.verify_binding());
    assert_eq!(ledger.head_calls.load(Ordering::SeqCst), 2);
    assert_eq!(ledger.read_calls.load(Ordering::SeqCst), 2);
}

#[test]
fn post_reservation_correct_permit_rejected_positive_control_is_exact_and_bound() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let mut rejected = effect.record().clone();
    rejected.state = HostEffectState::Reserved;
    rejected.outcome_sha256 = None;
    let ledger = PostReservationObservationLedger::stable(
        effect.permit().permit_id().to_owned(),
        rejected.current_head().clone(),
        rejected.clone(),
    );
    let failure = direct_post_reservation_failure(
        target,
        &ledger,
        &effect,
        vec![HostEffectExecutorErrorId::Timeout],
    );
    assert_eq!(failure.terminal_state(), None);
    let recovery = failure.recovery().unwrap();
    assert!(recovery.has_exact_current_ledger_observation());
    assert_eq!(recovery.ledger_record().unwrap(), &rejected);
    assert_eq!(
        recovery.post_reservation_ledger_classification(),
        Some(HostEffectPostReservationLedgerClassification::ObservationRejected)
    );
    assert_eq!(
        recovery.originating_error_ids(),
        &[HostEffectExecutorErrorId::Timeout]
    );
    assert!(recovery.outcome().is_none());
    assert!(recovery.observation().is_none());
    assert!(recovery.verify_binding());
    assert_eq!(ledger.head_calls.load(Ordering::SeqCst), 2);
    assert_eq!(ledger.read_calls.load(Ordering::SeqCst), 2);
}

#[test]
fn post_reservation_observation_error_falls_back_bound_unavailable_and_preserves_chain() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let ledger = PostReservationObservationLedger::read_error(
        effect.permit().permit_id().to_owned(),
        effect.record().current_head().clone(),
        effect.record().clone(),
    );
    let failure = direct_post_reservation_failure(
        target,
        &ledger,
        &effect,
        vec![HostEffectExecutorErrorId::Timeout],
    );
    assert_bound_unavailable_post_reservation(
        &failure,
        HostEffectExecutorErrorId::RecoveryRequired,
        &effect,
        &[
            HostEffectExecutorErrorId::Timeout,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ],
    );
    assert_eq!(ledger.head_calls.load(Ordering::SeqCst), 3);
    assert_eq!(ledger.read_calls.load(Ordering::SeqCst), 3);
}

#[test]
fn wrong_permit_post_reservation_replay_preflight_route_falls_back_bound_unavailable() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let replay_record = recovering_terminal_record(&durable, &effect);
    let wrong_permit_record = unrelated_record(
        &fixture,
        &durable,
        &target_identity,
        HostEffectState::InFlight,
    );
    let ledger = ReplayThenWrongPermitObservationLedger {
        expected_permit_id: effect.permit().permit_id().to_owned(),
        replay_record,
        observed_head: wrong_permit_record.current_head().clone(),
        wrong_permit_record,
        head_calls: AtomicU64::new(0),
        read_calls: AtomicU64::new(0),
    };
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let mut backend = ScriptedBackend::success();
    let mut executor = SupportedHostEffectExecutor::new(
        &ledger,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );
    let failure = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_bound_unavailable_post_reservation(
        &failure,
        HostEffectExecutorErrorId::Replay,
        &effect,
        &[HostEffectExecutorErrorId::Replay],
    );
    assert_eq!(ledger.head_calls.load(Ordering::SeqCst), 6);
    assert_eq!(ledger.read_calls.load(Ordering::SeqCst), 8);
    assert!(fs::read_dir(&fixture.target_root).unwrap().next().is_none());
    drop(executor);
    assert_eq!(backend.calls, 0);
}

#[test]
fn positive_actual_publication_fsync_rename_ledger_and_ack_transaction_settles() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let ledger =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &ledger, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let mut backend = ScriptedBackend::success();
    let policy = HostEffectExecutionPolicy::strict(10_000, &[]).unwrap();
    let mut executor = SupportedHostEffectExecutor::new(&ledger, target, &mut backend, policy);
    let receipt = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap();
    assert_eq!(
        receipt.publication_classification().id(),
        PublicationClassificationId::AcknowledgedCommitted
    );
    assert_eq!(receipt.command_output_sha256().len(), 1);
    assert_eq!(
        PublicationAcknowledgementIdentity::from_canonical_json(receipt.acknowledgement_json())
            .unwrap(),
        *receipt.acknowledgement()
    );
    let terminal = ledger.read(&permit_id).unwrap().unwrap();
    assert_eq!(terminal.state(), HostEffectState::Settled);
    assert_eq!(terminal.current_head(), receipt.terminal_ledger_head());
    let entries = fs::read_dir(&fixture.target_root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].starts_with("effect-"));
    let metadata = fs::symlink_metadata(fixture.target_root.join(&entries[0])).unwrap();
    assert!(metadata.is_file());
    assert_eq!(metadata.permissions().mode() & 0o777, 0o400);
    assert_eq!(metadata.nlink(), 1);

    let replay = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(replay.id(), HostEffectExecutorErrorId::Replay);
    assert_eq!(replay.terminal_state(), Some(HostEffectState::Settled));
    let replay_recovery = replay.recovery().unwrap();
    assert_eq!(replay_recovery.permit_id(), permit_id);
    assert!(replay_recovery.has_exact_current_ledger_observation());
    assert_eq!(
        replay_recovery.post_reservation_ledger_classification(),
        Some(HostEffectPostReservationLedgerClassification::TerminalObserved)
    );
    assert_eq!(
        replay_recovery.post_reservation_publication_classification(),
        Some(HostEffectPostReservationPublicationClassification::NoPublicationEvidence)
    );
    assert_eq!(
        replay_recovery.originating_error_ids(),
        &[HostEffectExecutorErrorId::Replay]
    );
    assert!(replay_recovery.verify_binding());
    drop(executor);
    assert_eq!(backend.calls, 1);
}

#[test]
fn negative_started_backend_terminal_failure_before_mutation_returns_exact_recovery() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let failing = FailTerminalLedger { inner: &durable };
    let mut backend = ScriptedBackend {
        replies: VecDeque::from([BackendReply::Failure(
            HostEffectExecutorErrorId::Timeout,
            true,
        )]),
        calls: 0,
    };
    let mut executor = SupportedHostEffectExecutor::new(
        &failing,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );
    let capability = capability();
    let expected_effect_identity = executor.effect_identity(&capability, &effect).unwrap();
    let failure = executor
        .execute_authorized_for_test(
            &capability,
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(failure.id(), HostEffectExecutorErrorId::RecoveryRequired);
    assert_ne!(failure.id(), HostEffectExecutorErrorId::LedgerSubstitution);
    assert_eq!(failure.terminal_state(), None);
    let recovery = failure.recovery().unwrap();
    assert_eq!(recovery.effect_identity_sha256(), expected_effect_identity);
    assert_eq!(recovery.permit_id(), permit_id);
    assert_eq!(recovery.ledger_head(), &durable.head().unwrap());
    assert_eq!(
        recovery.ledger_record().unwrap(),
        &durable.read(&permit_id).unwrap().unwrap()
    );
    assert!(recovery.has_exact_current_ledger_observation());
    assert_eq!(
        recovery.terminal_classification(),
        Some(HostEffectTerminalRecoveryClassification::StillInFlight)
    );
    assert_eq!(
        recovery.originating_error_ids(),
        &[
            HostEffectExecutorErrorId::Timeout,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ]
    );
    let outcome = recovery.outcome().unwrap();
    assert_eq!(outcome.permit_id, permit_id);
    assert_eq!(outcome.state, HostEffectState::Ambiguous);
    assert!(outcome.observed_post_state_sha256.is_none());
    assert!(recovery.binding_sha256().unwrap().starts_with("sha256:"));
    assert!(recovery.verify_binding());
    assert_eq!(
        durable.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::InFlight
    );
    assert!(fs::read_dir(&fixture.target_root).unwrap().next().is_none());
}

#[test]
fn negative_clock_failure_terminal_transition_also_returns_exact_recovery() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let failing = FailTerminalLedger { inner: &durable };
    let mut backend = ScriptedBackend::success();
    let mut executor = SupportedHostEffectExecutor::new(
        &failing,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );
    let failure = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut FailingClock,
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(failure.id(), HostEffectExecutorErrorId::RecoveryRequired);
    assert_eq!(failure.terminal_state(), None);
    let recovery = failure.recovery().unwrap();
    assert_eq!(
        recovery.terminal_classification(),
        Some(HostEffectTerminalRecoveryClassification::StillInFlight)
    );
    assert_eq!(
        recovery.originating_error_ids(),
        &[
            HostEffectExecutorErrorId::Io,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ]
    );
    assert_eq!(
        recovery.outcome().unwrap().state,
        HostEffectState::Ambiguous
    );
    assert_eq!(recovery.outcome().unwrap().command_output_sha256.len(), 1);
    assert!(recovery.has_exact_current_ledger_observation());
    assert!(recovery.verify_binding());
    assert_eq!(
        durable.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::InFlight
    );
    assert!(fs::read_dir(&fixture.target_root).unwrap().next().is_none());
}

#[test]
fn negative_preflight_ledger_substitution_after_handoff_is_identity_bearing() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let unavailable = UnavailableObservationLedger;
    let mut backend = ScriptedBackend::success();
    let mut executor = SupportedHostEffectExecutor::new(
        &unavailable,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );
    let capability = capability();
    let expected_effect_identity = executor.effect_identity(&capability, &effect).unwrap();
    let failure = executor
        .execute_authorized_for_test(
            &capability,
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(failure.id(), HostEffectExecutorErrorId::RecoveryRequired);
    assert_ne!(failure.id(), HostEffectExecutorErrorId::LedgerSubstitution);
    assert_eq!(failure.terminal_state(), None);
    let recovery = failure.recovery().unwrap();
    assert_eq!(recovery.effect_identity_sha256(), expected_effect_identity);
    assert_eq!(recovery.permit_id(), permit_id);
    assert_eq!(
        recovery.originating_error_id(),
        Some(HostEffectExecutorErrorId::LedgerSubstitution)
    );
    assert_eq!(
        recovery.terminal_classification(),
        Some(HostEffectTerminalRecoveryClassification::LedgerObservationUnavailable)
    );
    assert!(!recovery.has_exact_current_ledger_observation());
    assert_eq!(recovery.outcome().unwrap().state, HostEffectState::Failed);
    assert!(recovery.verify_binding());
    assert_eq!(backend.calls, 0);
    assert_eq!(
        durable.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::InFlight
    );
    assert!(fs::read_dir(&fixture.target_root).unwrap().next().is_none());
}

#[test]
fn race_committed_terminal_error_is_reobserved_as_committed_but_unverifiable() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let failing = CommitThenFailTerminalLedger { inner: &durable };
    let mut backend = ScriptedBackend {
        replies: VecDeque::from([BackendReply::Failure(
            HostEffectExecutorErrorId::Timeout,
            true,
        )]),
        calls: 0,
    };
    let mut executor = SupportedHostEffectExecutor::new(
        &failing,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );
    let failure = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(failure.id(), HostEffectExecutorErrorId::RecoveryRequired);
    assert_eq!(failure.terminal_state(), Some(HostEffectState::Ambiguous));
    let recovery = failure.recovery().unwrap();
    assert!(recovery.has_exact_current_ledger_observation());
    assert_eq!(recovery.ledger_head(), &durable.head().unwrap());
    assert_eq!(
        recovery.terminal_classification(),
        Some(HostEffectTerminalRecoveryClassification::TerminalCommittedButUnverifiable)
    );
    assert_eq!(
        recovery.outcome().unwrap().state,
        HostEffectState::Ambiguous
    );
    assert!(recovery.verify_binding());
    assert_eq!(
        durable.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::Ambiguous
    );
    assert!(fs::read_dir(&fixture.target_root).unwrap().next().is_none());
}

fn assert_unrelated_head_advance_returns_bound_unavailable_recovery(
    observation_mode: RecoveryObservationMode,
    commit_terminal: bool,
    repeat_advance: bool,
) {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let reservation_head = effect.record().current_head().clone();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let advancing = AdvanceUnrelatedPermitThenFailTerminalLedger {
        inner: &durable,
        authority: HostEffectAuthority::generate(
            "unrelated-root".to_owned(),
            "fixture-ledger".to_owned(),
        )
        .unwrap(),
        unrelated_binding: permit_binding(&fixture, &durable, &target_identity, '0'),
        stale_head: reservation_head.clone(),
        commit_terminal,
        repeat_advance,
        observation_mode,
        advanced: AtomicBool::new(false),
        unrelated_record: Mutex::new(None),
    };
    let mut backend = ScriptedBackend {
        replies: VecDeque::from([BackendReply::Failure(
            HostEffectExecutorErrorId::Timeout,
            true,
        )]),
        calls: 0,
    };
    let mut executor = SupportedHostEffectExecutor::new(
        &advancing,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );
    let failure = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(failure.id(), HostEffectExecutorErrorId::RecoveryRequired);
    assert_eq!(failure.terminal_state(), None);
    let recovery = failure.recovery().unwrap();
    assert_eq!(recovery.permit_id(), permit_id);
    assert_eq!(recovery.ledger_head(), &reservation_head);
    assert_eq!(recovery.ledger_record().unwrap(), effect.record());
    assert!(!recovery.has_exact_current_ledger_observation());
    assert_eq!(
        recovery.terminal_classification(),
        Some(HostEffectTerminalRecoveryClassification::LedgerObservationUnavailable)
    );
    assert_eq!(
        recovery.originating_error_ids(),
        &[
            HostEffectExecutorErrorId::Timeout,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ]
    );
    assert_eq!(
        recovery.outcome().unwrap().state,
        HostEffectState::Ambiguous
    );
    assert!(recovery.observation().is_none());
    assert!(recovery.prior_publication_observation().is_none());
    assert!(recovery.publication_identity_sha256().is_none());
    assert!(recovery.classification().is_none());
    assert!(recovery.verify_binding());

    let global_head_after = durable.head().unwrap();
    assert_ne!(global_head_after, reservation_head);
    let mut mismatched = recovery.clone();
    mismatched.substitute_ledger_head_without_rebinding_for_test(global_head_after);
    assert!(!mismatched.verify_binding());
    assert_eq!(
        durable.read(&permit_id).unwrap().unwrap().state(),
        if commit_terminal {
            HostEffectState::Ambiguous
        } else {
            HostEffectState::InFlight
        }
    );
    assert!(fs::read_dir(&fixture.target_root).unwrap().next().is_none());
    drop(executor);
    assert_eq!(backend.calls, 1);
}

#[test]
fn race_unrelated_permit_global_head_advance_with_still_in_flight_record_is_unavailable_and_bound()
{
    assert_unrelated_head_advance_returns_bound_unavailable_recovery(
        RecoveryObservationMode::Current,
        false,
        false,
    );
}

#[test]
fn race_repeated_unrelated_head_advances_after_terminal_commit_withhold_terminal_classification() {
    assert_unrelated_head_advance_returns_bound_unavailable_recovery(
        RecoveryObservationMode::Current,
        true,
        true,
    );
}

#[test]
fn negative_unrelated_head_advance_record_substitution_is_unavailable_and_bound() {
    assert_unrelated_head_advance_returns_bound_unavailable_recovery(
        RecoveryObservationMode::SubstituteUnrelatedRecord,
        false,
        false,
    );
}

#[test]
fn negative_head_rollback_after_terminal_and_unrelated_advance_is_unavailable_and_bound() {
    assert_unrelated_head_advance_returns_bound_unavailable_recovery(
        RecoveryObservationMode::StaleHead,
        true,
        false,
    );
}

#[test]
fn negative_observation_error_after_unrelated_head_advance_is_unavailable_and_bound() {
    assert_unrelated_head_advance_returns_bound_unavailable_recovery(
        RecoveryObservationMode::Error,
        false,
        false,
    );
}

#[test]
fn security_mutated_terminal_record_is_rejected_without_false_terminal_claim() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let failing = MutateCommitThenFailTerminalLedger { inner: &durable };
    let mut backend = ScriptedBackend {
        replies: VecDeque::from([BackendReply::Failure(
            HostEffectExecutorErrorId::Timeout,
            true,
        )]),
        calls: 0,
    };
    let mut executor = SupportedHostEffectExecutor::new(
        &failing,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );
    let failure = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(failure.id(), HostEffectExecutorErrorId::RecoveryRequired);
    assert_eq!(failure.terminal_state(), None);
    let recovery = failure.recovery().unwrap();
    assert_eq!(
        recovery.terminal_classification(),
        Some(HostEffectTerminalRecoveryClassification::LedgerObservationRejected)
    );
    assert_eq!(
        recovery.outcome().unwrap().state,
        HostEffectState::Ambiguous
    );
    assert_eq!(
        recovery.ledger_record().unwrap().state(),
        HostEffectState::Failed
    );
    assert!(recovery.verify_binding());
    assert_eq!(
        durable.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::Failed
    );
}

#[test]
fn mutation_of_recovery_permit_binding_is_detected_as_false_pass() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let failing = FailTerminalLedger { inner: &durable };
    let mut backend = ScriptedBackend {
        replies: VecDeque::from([BackendReply::Failure(
            HostEffectExecutorErrorId::ProcessSpawnFailed,
            false,
        )]),
        calls: 0,
    };
    let mut executor = SupportedHostEffectExecutor::new(
        &failing,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );
    let failure = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    let mut recovery = failure.recovery().unwrap().clone();
    assert!(recovery.verify_binding());
    recovery.substitute_permit_without_rebinding_for_test(digest('0'));
    assert!(!recovery.verify_binding());
}

#[test]
fn mutation_of_recovery_error_chain_order_length_and_members_is_detected() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let failing = FailTerminalLedger { inner: &durable };
    let mut backend = ScriptedBackend {
        replies: VecDeque::from([BackendReply::Failure(
            HostEffectExecutorErrorId::ProcessSpawnFailed,
            false,
        )]),
        calls: 0,
    };
    let mut executor = SupportedHostEffectExecutor::new(
        &failing,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );
    let failure = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    let recovery = failure.recovery().unwrap();
    let expected = [
        HostEffectExecutorErrorId::ProcessSpawnFailed,
        HostEffectExecutorErrorId::LedgerSubstitution,
    ];
    assert_eq!(recovery.originating_error_ids(), &expected);
    assert!(recovery.verify_binding());

    for replacement in [
        vec![
            HostEffectExecutorErrorId::Timeout,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ],
        vec![
            HostEffectExecutorErrorId::LedgerSubstitution,
            HostEffectExecutorErrorId::ProcessSpawnFailed,
        ],
        vec![HostEffectExecutorErrorId::ProcessSpawnFailed],
        vec![
            HostEffectExecutorErrorId::ProcessSpawnFailed,
            HostEffectExecutorErrorId::LedgerSubstitution,
            HostEffectExecutorErrorId::LedgerSubstitution,
        ],
        vec![
            HostEffectExecutorErrorId::ProcessSpawnFailed,
            HostEffectExecutorErrorId::LedgerSubstitution,
            HostEffectExecutorErrorId::RecoveryRequired,
        ],
    ] {
        let mut mutated = recovery.clone();
        mutated.substitute_originating_error_ids_without_rebinding_for_test(replacement);
        assert!(!mutated.verify_binding());
    }
}

#[test]
fn committed_publication_with_failed_terminal_transition_requires_recovery() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let failing = FailTerminalLedger { inner: &durable };
    let mut backend = ScriptedBackend::success();
    let policy = HostEffectExecutionPolicy::strict(10_000, &[]).unwrap();
    let mut executor = SupportedHostEffectExecutor::new(&failing, target, &mut backend, policy);
    let failure = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(failure.id(), HostEffectExecutorErrorId::RecoveryRequired);
    assert_eq!(failure.terminal_state(), None);
    assert_eq!(
        failure.recovery().unwrap().classification().unwrap().id(),
        PublicationClassificationId::CommittedBeforeAcknowledgement
    );
    assert_eq!(
        durable.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::InFlight
    );
    assert_eq!(fs::read_dir(&fixture.target_root).unwrap().count(), 1);
}

#[test]
fn publication_failure_terminal_commit_then_reobservation_loss_retains_identity() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let displaced_root = PathBuf::from(format!(
        "{}-publication-terminal-displaced",
        fixture.target_root.to_string_lossy()
    ));
    let transition_calls = Arc::new(AtomicU64::new(0));
    let committing = CommitThenDisplaceTargetLedger {
        inner: &durable,
        target_root: fixture.target_root.clone(),
        displaced_root: displaced_root.clone(),
        transition_calls: Arc::clone(&transition_calls),
    };
    set_fault(FaultPoint::BeforeTempFsync, true, |_| {});
    let mut backend = ScriptedBackend::success();
    let mut executor = SupportedHostEffectExecutor::new(
        &committing,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );
    let capability = capability();
    let expected_effect_identity = executor.effect_identity(&capability, &effect).unwrap();
    let failure = executor
        .execute_authorized_for_test(
            &capability,
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();

    assert_eq!(failure.id(), HostEffectExecutorErrorId::RecoveryRequired);
    assert_eq!(failure.terminal_state(), Some(HostEffectState::Ambiguous));
    let recovery = failure.recovery().unwrap();
    assert_eq!(recovery.effect_identity_sha256(), expected_effect_identity);
    assert_eq!(recovery.permit_id(), permit_id);
    assert_eq!(
        recovery.originating_error_ids(),
        &[
            HostEffectExecutorErrorId::SyncFailure,
            HostEffectExecutorErrorId::PathSwap,
        ]
    );
    assert_eq!(
        recovery.post_reservation_ledger_classification(),
        Some(HostEffectPostReservationLedgerClassification::TerminalObserved)
    );
    assert_eq!(
        recovery.post_reservation_publication_classification(),
        Some(
            HostEffectPostReservationPublicationClassification::PriorObservationCurrentObservationUnavailable
        )
    );
    assert!(recovery.has_exact_current_ledger_observation());
    assert!(!recovery.has_exact_current_publication_observation());
    assert!(
        recovery
            .prior_publication_observation()
            .unwrap()
            .classify()
            .is_ok()
    );
    assert!(recovery.observation().is_none());
    assert!(recovery.classification().is_none());
    assert!(recovery.verify_binding());
    let mut substituted = recovery.clone();
    substituted.substitute_permit_without_rebinding_for_test(digest('0'));
    assert!(!substituted.verify_binding());
    assert_eq!(transition_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        durable.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::Ambiguous
    );
    assert!(!fixture.target_root.exists());
    assert_eq!(fs::read_dir(&displaced_root).unwrap().count(), 1);

    drop(executor);
    assert_eq!(backend.calls, 1);
    drop(lease);
    drop(observer);
    fs::remove_dir_all(displaced_root).unwrap();
}

#[test]
fn committed_publication_terminal_commit_then_reobservation_loss_retains_identity() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let displaced_root = PathBuf::from(format!(
        "{}-committed-terminal-displaced",
        fixture.target_root.to_string_lossy()
    ));
    let transition_calls = Arc::new(AtomicU64::new(0));
    let committing = CommitThenDisplaceTargetLedger {
        inner: &durable,
        target_root: fixture.target_root.clone(),
        displaced_root: displaced_root.clone(),
        transition_calls: Arc::clone(&transition_calls),
    };
    let mut backend = ScriptedBackend::success();
    let mut executor = SupportedHostEffectExecutor::new(
        &committing,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );
    let capability = capability();
    let expected_effect_identity = executor.effect_identity(&capability, &effect).unwrap();
    let failure = executor
        .execute_authorized_for_test(
            &capability,
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();

    assert_eq!(failure.id(), HostEffectExecutorErrorId::FalsePassReceipt);
    assert_eq!(failure.terminal_state(), Some(HostEffectState::Settled));
    let recovery = failure.recovery().unwrap();
    assert_eq!(recovery.effect_identity_sha256(), expected_effect_identity);
    assert_eq!(recovery.permit_id(), permit_id);
    assert_eq!(
        recovery.originating_error_ids(),
        &[
            HostEffectExecutorErrorId::FalsePassReceipt,
            HostEffectExecutorErrorId::PathSwap,
        ]
    );
    assert_eq!(
        recovery.post_reservation_ledger_classification(),
        Some(HostEffectPostReservationLedgerClassification::TerminalObserved)
    );
    assert_eq!(
        recovery.post_reservation_publication_classification(),
        Some(
            HostEffectPostReservationPublicationClassification::PriorObservationCurrentObservationUnavailable
        )
    );
    assert!(recovery.has_exact_current_ledger_observation());
    assert!(!recovery.has_exact_current_publication_observation());
    assert_eq!(
        recovery
            .prior_publication_observation()
            .unwrap()
            .classify()
            .unwrap()
            .id(),
        PublicationClassificationId::CommittedBeforeAcknowledgement
    );
    assert!(recovery.outcome().is_none());
    assert!(recovery.verify_binding());
    assert_eq!(transition_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        durable.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::Settled
    );
    assert!(!fixture.target_root.exists());
    assert_eq!(fs::read_dir(&displaced_root).unwrap().count(), 1);

    drop(executor);
    assert_eq!(backend.calls, 1);
    drop(lease);
    drop(observer);
    fs::remove_dir_all(displaced_root).unwrap();
}

#[test]
fn publication_failure_terminal_io_then_reobservation_loss_retains_identity() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let displaced_root = PathBuf::from(format!(
        "{}-publication-transition-io-displaced",
        fixture.target_root.to_string_lossy()
    ));
    let transition_calls = Arc::new(AtomicU64::new(0));
    let failing = DisplaceTargetAndFailTerminalLedger {
        inner: &durable,
        target_root: fixture.target_root.clone(),
        displaced_root: displaced_root.clone(),
        transition_calls: Arc::clone(&transition_calls),
    };
    set_fault(FaultPoint::BeforeTempFsync, true, |_| {});
    let mut backend = ScriptedBackend::success();
    let mut executor = SupportedHostEffectExecutor::new(
        &failing,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );
    let capability = capability();
    let expected_effect_identity = executor.effect_identity(&capability, &effect).unwrap();
    let failure = executor
        .execute_authorized_for_test(
            &capability,
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();

    assert_eq!(failure.id(), HostEffectExecutorErrorId::RecoveryRequired);
    assert_eq!(failure.terminal_state(), None);
    let recovery = failure.recovery().unwrap();
    assert_eq!(recovery.effect_identity_sha256(), expected_effect_identity);
    assert_eq!(recovery.permit_id(), permit_id);
    assert_eq!(
        recovery.originating_error_ids(),
        &[
            HostEffectExecutorErrorId::SyncFailure,
            HostEffectExecutorErrorId::LedgerSubstitution,
            HostEffectExecutorErrorId::PathSwap,
        ]
    );
    assert_eq!(
        recovery.post_reservation_ledger_classification(),
        Some(HostEffectPostReservationLedgerClassification::StillInFlight)
    );
    assert_eq!(
        recovery.post_reservation_publication_classification(),
        Some(
            HostEffectPostReservationPublicationClassification::PriorObservationCurrentObservationUnavailable
        )
    );
    assert!(recovery.has_exact_current_ledger_observation());
    assert!(!recovery.has_exact_current_publication_observation());
    assert_eq!(
        recovery.ledger_record().unwrap().state(),
        HostEffectState::InFlight
    );
    assert!(
        recovery
            .prior_publication_observation()
            .unwrap()
            .classify()
            .is_ok()
    );
    assert!(recovery.observation().is_none());
    assert!(recovery.classification().is_none());
    assert!(recovery.outcome().is_none());
    assert!(recovery.verify_binding());
    assert_eq!(transition_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        durable.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::InFlight
    );
    assert!(!fixture.target_root.exists());
    assert_eq!(fs::read_dir(&displaced_root).unwrap().count(), 1);

    drop(executor);
    assert_eq!(backend.calls, 1);
    drop(lease);
    drop(observer);
    fs::remove_dir_all(displaced_root).unwrap();
}

#[test]
fn dual_failure_after_committed_publication_retains_identity_when_reobservation_is_unavailable() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let prior_ledger_head = effect.record().current_head().clone();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let displaced_root = PathBuf::from(format!(
        "{}-displaced",
        fixture.target_root.to_string_lossy()
    ));
    let transition_calls = Arc::new(AtomicU64::new(0));
    let failing = DisplaceTargetAndFailTerminalLedger {
        inner: &durable,
        target_root: fixture.target_root.clone(),
        displaced_root: displaced_root.clone(),
        transition_calls: Arc::clone(&transition_calls),
    };
    let mut backend = ScriptedBackend::success();
    let mut executor = SupportedHostEffectExecutor::new(
        &failing,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );
    let capability = capability();
    let expected_effect_identity = executor.effect_identity(&capability, &effect).unwrap();
    let failure = executor
        .execute_authorized_for_test(
            &capability,
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();

    assert_eq!(failure.id(), HostEffectExecutorErrorId::RecoveryRequired);
    assert_eq!(failure.terminal_state(), None);
    let recovery = failure.recovery().unwrap();
    assert_eq!(recovery.effect_identity_sha256(), expected_effect_identity);
    assert_eq!(recovery.permit_id(), permit_id);
    assert_eq!(recovery.ledger_head(), &prior_ledger_head);
    assert!(
        recovery
            .publication_identity_sha256()
            .unwrap()
            .starts_with("sha256:")
    );
    assert_eq!(
        recovery.originating_error_ids(),
        &[
            HostEffectExecutorErrorId::LedgerSubstitution,
            HostEffectExecutorErrorId::PathSwap,
        ]
    );
    assert_eq!(
        recovery.post_reservation_ledger_classification(),
        Some(HostEffectPostReservationLedgerClassification::StillInFlight)
    );
    assert_eq!(
        recovery.post_reservation_publication_classification(),
        Some(
            HostEffectPostReservationPublicationClassification::PriorObservationCurrentObservationUnavailable
        )
    );
    assert_eq!(
        recovery
            .prior_publication_observation()
            .unwrap()
            .classify()
            .unwrap()
            .id(),
        PublicationClassificationId::CommittedBeforeAcknowledgement
    );
    assert!(recovery.observation().is_none());
    assert!(recovery.classification().is_none());
    assert!(!recovery.has_exact_current_publication_observation());
    assert!(recovery.has_exact_current_ledger_observation());
    assert!(recovery.terminal_classification().is_none());
    assert_eq!(
        recovery.ledger_record().unwrap().state(),
        HostEffectState::InFlight
    );
    assert!(recovery.outcome().is_none());
    assert!(recovery.binding_sha256().unwrap().starts_with("sha256:"));
    assert!(recovery.verify_binding());
    assert_eq!(transition_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        durable.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::InFlight
    );
    assert!(!fixture.target_root.exists());
    assert_eq!(fs::read_dir(&displaced_root).unwrap().count(), 1);

    drop(executor);
    assert_eq!(backend.calls, 1);
    drop(lease);
    drop(observer);
    fs::remove_dir_all(displaced_root).unwrap();
}

#[test]
fn interrupted_publication_with_failed_terminal_transition_requires_recovery() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let durable =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &durable, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    set_fault(FaultPoint::BeforeTempFsync, true, |_| {});
    let failing = FailTerminalLedger { inner: &durable };
    let mut backend = ScriptedBackend::success();
    let mut executor = SupportedHostEffectExecutor::new(
        &failing,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );
    let failure = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(failure.id(), HostEffectExecutorErrorId::RecoveryRequired);
    assert_eq!(failure.terminal_state(), None);
    assert!(failure.recovery().is_some());
    assert_eq!(
        durable.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::InFlight
    );
    assert_eq!(fs::read_dir(&fixture.target_root).unwrap().count(), 1);
}

#[test]
fn directory_fsync_failure_is_ambiguous_and_hands_off_exact_recovery() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let ledger =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &ledger, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let reached = Arc::new(AtomicBool::new(false));
    let reached_by_hook = Arc::clone(&reached);
    set_fault(FaultPoint::BeforeDirectoryFsync, true, move |_| {
        reached_by_hook.store(true, Ordering::SeqCst);
    });
    let mut backend = ScriptedBackend::success();
    let policy = HostEffectExecutionPolicy::strict(10_000, &[]).unwrap();
    let mut executor = SupportedHostEffectExecutor::new(&ledger, target, &mut backend, policy);
    let failure = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert!(reached.load(Ordering::SeqCst));
    assert_eq!(failure.id(), HostEffectExecutorErrorId::SyncFailure);
    assert_eq!(failure.terminal_state(), Some(HostEffectState::Ambiguous));
    let recovery = failure.recovery().unwrap();
    assert_eq!(recovery.permit_id(), permit_id);
    assert_eq!(*recovery.ledger_head(), ledger.head().unwrap());
    assert_eq!(
        ledger.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::Ambiguous
    );
}

#[test]
fn rename_race_and_output_overflow_never_settle() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let ledger =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &ledger, &target_identity);
    let target_name = receipt_name(effect.permit().permit_id()).unwrap();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    set_fault(FaultPoint::BeforeRename, false, move |root| {
        let path = root.join(&target_name);
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .unwrap();
        file.write_all(b"racer").unwrap();
        file.sync_all().unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o400)).unwrap();
    });
    let mut backend = ScriptedBackend::success();
    let policy = HostEffectExecutionPolicy::strict(10_000, &[]).unwrap();
    let mut executor = SupportedHostEffectExecutor::new(&ledger, target, &mut backend, policy);
    let failure = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(failure.id(), HostEffectExecutorErrorId::RenameRace);
    assert_eq!(failure.terminal_state(), Some(HostEffectState::Ambiguous));

    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let ledger =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &ledger, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let mut backend = ScriptedBackend {
        replies: VecDeque::from([BackendReply::Oversized]),
        calls: 0,
    };
    let policy = HostEffectExecutionPolicy::strict(10_000, &[]).unwrap();
    let mut executor = SupportedHostEffectExecutor::new(&ledger, target, &mut backend, policy);
    let failure = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(failure.id(), HostEffectExecutorErrorId::OutputOverflow);
    assert_eq!(
        ledger.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::Ambiguous
    );
}

#[test]
fn environment_injection_and_prestart_cancel_fail_without_backend_effect() {
    assert_eq!(
        HostEffectExecutionPolicy::strict(
            10_000,
            &[("DYLD_INSERT_LIBRARIES".to_owned(), "/tmp/inject".to_owned())]
        )
        .unwrap_err()
        .id(),
        HostEffectExecutorErrorId::EnvironmentInjection
    );

    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let ledger =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &ledger, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let mut backend = ScriptedBackend::success();
    let cancellation = HostEffectCancellation::default();
    cancellation.cancel();
    let policy = HostEffectExecutionPolicy::strict(10_000, &[]).unwrap();
    let mut executor = SupportedHostEffectExecutor::new(&ledger, target, &mut backend, policy);
    let failure = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &cancellation,
        )
        .unwrap_err();
    assert_eq!(failure.id(), HostEffectExecutorErrorId::Cancelled);
    assert_eq!(failure.terminal_state(), Some(HostEffectState::Failed));
    assert_eq!(backend.calls, 0);
    assert_eq!(
        ledger.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::Failed
    );
    assert!(fs::read_dir(&fixture.target_root).unwrap().next().is_none());
}

#[test]
fn executable_mutation_and_unsafe_namespace_objects_fail_before_backend_effect() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let ledger =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &ledger, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut changed = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&fixture.executable)
        .unwrap();
    changed.write_all(b"#!/bin/sh\nexit 17\n").unwrap();
    changed.sync_all().unwrap();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let mut backend = ScriptedBackend::success();
    let mut executor = SupportedHostEffectExecutor::new(
        &ledger,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );
    let failure = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(failure.id(), HostEffectExecutorErrorId::ExecutableMutation);
    assert_eq!(failure.terminal_state(), Some(HostEffectState::Failed));
    drop(executor);
    assert_eq!(backend.calls, 0);
    assert_eq!(
        ledger.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::Failed
    );

    for object in ["symlink", "hardlink", "fifo", "temp-collision"] {
        let fixture = Fixture::new();
        let (_scope, target, _target_identity) = fixture.scope_and_target();
        let target_name = format!("effect-{}.json", "a".repeat(64));
        let path = fixture.target_root.join(&target_name);
        match object {
            "symlink" => std::os::unix::fs::symlink("/private/tmp", &path).unwrap(),
            "hardlink" => {
                let source = fixture.target_root.join("unrelated-source");
                let mut file = OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&source)
                    .unwrap();
                file.write_all(b"linked").unwrap();
                file.sync_all().unwrap();
                fs::hard_link(source, &path).unwrap();
            }
            "fifo" => {
                let path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
            }
            "temp-collision" => {
                let temp = fixture
                    .target_root
                    .join(format!(".{target_name}.0000000000000000.tmp"));
                let file = OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(temp)
                    .unwrap();
                file.sync_all().unwrap();
            }
            _ => unreachable!(),
        }
        let failure = target
            .require_clean_publication_name(&target_name)
            .unwrap_err();
        let expected = if object == "temp-collision" {
            HostEffectExecutorErrorId::TempCollision
        } else {
            HostEffectExecutorErrorId::UnsafeObject
        };
        assert_eq!(failure.id(), expected, "object={object}");
    }
}

#[test]
fn started_timeout_is_ambiguous_and_never_publishes() {
    let fixture = Fixture::new();
    let (_scope, target, target_identity) = fixture.scope_and_target();
    let ledger =
        FileHostEffectLedger::create(&fixture.ledger_root, "fixture-ledger".to_owned()).unwrap();
    let effect = authorized_effect(&fixture, &ledger, &target_identity);
    let permit_id = effect.permit().permit_id().to_owned();
    let mut observer = target.observer();
    let mut lease = observer.acquire(&target_identity).unwrap();
    let mut backend = ScriptedBackend {
        replies: VecDeque::from([BackendReply::Failure(
            HostEffectExecutorErrorId::Timeout,
            true,
        )]),
        calls: 0,
    };
    let mut executor = SupportedHostEffectExecutor::new(
        &ledger,
        target,
        &mut backend,
        HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
    );
    let failure = executor
        .execute_authorized_for_test(
            &capability(),
            &effect,
            lease.as_mut(),
            &mut TestClock { sequence: 0 },
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(failure.id(), HostEffectExecutorErrorId::Timeout);
    assert_eq!(failure.terminal_state(), Some(HostEffectState::Ambiguous));
    assert_eq!(
        ledger.read(&permit_id).unwrap().unwrap().state(),
        HostEffectState::Ambiguous
    );
    assert!(fs::read_dir(&fixture.target_root).unwrap().next().is_none());
}

#[cfg(target_os = "macos")]
#[test]
fn native_darwin_backend_refuses_before_spawn_or_output() {
    let fixture = Fixture::new();
    let executable = PinnedHostExecutable::pin(&fixture.executable).unwrap();
    let plan =
        HostCommandPlan::personal_install(&fixture.package(), "fixture-marketplace").unwrap();
    let mut backend = NativeRetainedDescriptorProcessBackend;
    let failure = backend
        .execute(
            &capability(),
            &executable,
            &plan.commands()[0],
            &HostEffectExecutionPolicy::strict(10_000, &[]).unwrap(),
            &HostEffectCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(failure.id, HostEffectExecutorErrorId::UnsupportedPlatform);
    assert!(!failure.started);
    assert!(failure.capture.stdout.is_empty());
    assert!(failure.capture.stderr.is_empty());
}
