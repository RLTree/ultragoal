use super::super::{
    AuthorizedHostEffect, DurableHostEffectLedger, HostEffectAuthority, HostEffectLedgerHead,
    HostEffectState, HostEffectTransition, PinnedHostExecutable,
};
use super::binding::{
    AcceptedHostEffect, AcceptedHostScope, AcceptedLifecyclePlan, ObservedTargetIdentity,
    RootPlanCustody,
};
use super::recovery::{
    PublicationClassification, RecoveryAuthorization, RecoveryProposal,
    issue_recovery_authorization, propose_recovery,
};
use super::{SupportedHostLifecycleError, SupportedHostLifecycleErrorId, lifecycle_error};
use crate::distribution::{
    HostCapabilityDeclaration, HostCommandPlan, JourneyBinding, PackageIdentity,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const AUTHORITY_NONCE_BYTES: usize = 32;
const LEDGER_NONCE_BYTES: usize = 32;
const MAX_PERMIT_TTL_MS: u64 = 5 * 60 * 1000;
const MAX_CLOCK_STEP_MS: u64 = 30_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum DescriptorExecutionPlatform {
    Darwin,
    Linux,
    FreeBsd,
    Other,
}

impl DescriptorExecutionPlatform {
    pub(crate) const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Darwin
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else if cfg!(target_os = "freebsd") {
            Self::FreeBsd
        } else {
            Self::Other
        }
    }

    const fn supports_descriptor_execution(self) -> bool {
        matches!(self, Self::Linux | Self::FreeBsd)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum DescriptorExecutionPrimitive {
    ExecveAtEmptyPath,
    Fexecve,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct DescriptorExecutionCapability {
    platform: DescriptorExecutionPlatform,
    primitive: DescriptorExecutionPrimitive,
    adapter_name: String,
    adapter_version: String,
    capability_sha256: String,
}

impl DescriptorExecutionCapability {
    pub(in crate::distribution::host_effect) fn new(
        platform: DescriptorExecutionPlatform,
        primitive: DescriptorExecutionPrimitive,
        adapter_name: String,
        adapter_version: String,
    ) -> Result<Self, SupportedHostLifecycleError> {
        if !matches!(
            (platform, primitive),
            (
                DescriptorExecutionPlatform::Linux,
                DescriptorExecutionPrimitive::ExecveAtEmptyPath
            ) | (
                DescriptorExecutionPlatform::FreeBsd,
                DescriptorExecutionPrimitive::Fexecve
            )
        ) || !valid_id(&adapter_name)
            || !valid_id(&adapter_version)
        {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::DescriptorExecutionUnavailable,
            ));
        }
        #[derive(Serialize)]
        struct Capability<'a> {
            schema: &'static str,
            platform: DescriptorExecutionPlatform,
            primitive: DescriptorExecutionPrimitive,
            adapter_name: &'a str,
            adapter_version: &'a str,
        }
        let capability_sha256 = digest_json(&Capability {
            schema: "harness-ultragoal.descriptor-execution-capability.v1",
            platform,
            primitive,
            adapter_name: &adapter_name,
            adapter_version: &adapter_version,
        })?;
        Ok(Self {
            platform,
            primitive,
            adapter_name,
            adapter_version,
            capability_sha256,
        })
    }

    pub(crate) const fn platform(&self) -> DescriptorExecutionPlatform {
        self.platform
    }

    pub(crate) const fn primitive(&self) -> DescriptorExecutionPrimitive {
        self.primitive
    }

    pub(crate) fn capability_sha256(&self) -> &str {
        &self.capability_sha256
    }
}

/// Defines the descriptor-execution capability handoff. There is deliberately
/// no production implementation in this candidate and no spawn method here.
pub(crate) trait DescriptorExecutionAdapter {
    fn descriptor_capability(
        &mut self,
    ) -> Result<DescriptorExecutionCapability, SupportedHostLifecycleError>;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct TrustedTimeSample {
    source_name: String,
    source_epoch: u64,
    sequence: u64,
    unix_ms: u64,
    attestation_sha256: String,
}

impl TrustedTimeSample {
    pub(in crate::distribution::host_effect) fn new(
        source_name: String,
        source_epoch: u64,
        sequence: u64,
        unix_ms: u64,
    ) -> Result<Self, SupportedHostLifecycleError> {
        if !valid_id(&source_name) || source_epoch == 0 || sequence == 0 || unix_ms == 0 {
            return Err(untrusted_time());
        }
        #[derive(Serialize)]
        struct Attestation<'a> {
            schema: &'static str,
            source_name: &'a str,
            source_epoch: u64,
            sequence: u64,
            unix_ms: u64,
        }
        let attestation_sha256 = digest_json(&Attestation {
            schema: "harness-ultragoal.trusted-time-sample.v1",
            source_name: &source_name,
            source_epoch,
            sequence,
            unix_ms,
        })?;
        Ok(Self {
            source_name,
            source_epoch,
            sequence,
            unix_ms,
            attestation_sha256,
        })
    }

    pub(crate) const fn unix_ms(&self) -> u64 {
        self.unix_ms
    }

    fn require_successor(&self, next: &Self) -> Result<(), SupportedHostLifecycleError> {
        if self.source_name != next.source_name
            || self.source_epoch != next.source_epoch
            || next.sequence <= self.sequence
            || next.unix_ms < self.unix_ms
            || next.unix_ms.saturating_sub(self.unix_ms) > MAX_CLOCK_STEP_MS
            || self.attestation_sha256 != recompute_time_attestation(self)?
            || next.attestation_sha256 != recompute_time_attestation(next)?
        {
            return Err(untrusted_time());
        }
        Ok(())
    }
}

/// A root-selected source must provide monotonic, externally trusted samples.
/// This module intentionally supplies no wall-clock implementation.
pub(crate) trait RootTrustedClock {
    fn sample(&mut self) -> Result<TrustedTimeSample, SupportedHostLifecycleError>;
}

/// A supported adapter must retain the exact target object and its generation
/// through handoff. Snapshot-only implementations do not satisfy this trait's
/// contract.
pub(crate) trait HostTargetLease: Send {
    fn identity(&self) -> &ObservedTargetIdentity;

    fn revalidate(&mut self) -> Result<ObservedTargetIdentity, SupportedHostLifecycleError>;
}

pub(crate) trait HostTargetObserver {
    fn acquire(
        &mut self,
        expected: &ObservedTargetIdentity,
    ) -> Result<Box<dyn HostTargetLease>, SupportedHostLifecycleError>;
}

struct BoundAuthority {
    authority: HostEffectAuthority,
    issuer_id: String,
    ledger_id: String,
    binding_sha256: String,
}

impl BoundAuthority {
    fn generate(issuer_id: String, ledger_id: String) -> Result<Self, SupportedHostLifecycleError> {
        let authority = HostEffectAuthority::generate(issuer_id.clone(), ledger_id.clone())
            .map_err(|_| authority_rejected())?;
        let mut nonce = [0_u8; AUTHORITY_NONCE_BYTES];
        getrandom::fill(&mut nonce).map_err(|_| authority_rejected())?;
        #[derive(Serialize)]
        struct Binding<'a> {
            schema: &'static str,
            issuer_id: &'a str,
            ledger_id: &'a str,
            instance_nonce_sha256: String,
        }
        let binding_sha256 = digest_json(&Binding {
            schema: "harness-ultragoal.root-host-effect-authority-instance.v1",
            issuer_id: &issuer_id,
            ledger_id: &ledger_id,
            instance_nonce_sha256: digest_bytes(&nonce),
        })?;
        nonce.fill(0);
        Ok(Self {
            authority,
            issuer_id,
            ledger_id,
            binding_sha256,
        })
    }
}

struct BoundLedger<'a> {
    ledger: &'a dyn DurableHostEffectLedger,
    ledger_id: String,
    binding_sha256: String,
}

impl<'a> BoundLedger<'a> {
    fn bind(
        ledger: &'a dyn DurableHostEffectLedger,
        ledger_id: String,
    ) -> Result<Self, SupportedHostLifecycleError> {
        if !valid_id(&ledger_id) {
            return Err(invalid());
        }
        let mut nonce = [0_u8; LEDGER_NONCE_BYTES];
        getrandom::fill(&mut nonce).map_err(|_| invalid())?;
        #[derive(Serialize)]
        struct Binding<'a> {
            schema: &'static str,
            ledger_id: &'a str,
            instance_nonce_sha256: String,
        }
        let binding_sha256 = digest_json(&Binding {
            schema: "harness-ultragoal.bound-host-effect-ledger-instance.v1",
            ledger_id: &ledger_id,
            instance_nonce_sha256: digest_bytes(&nonce),
        })?;
        nonce.fill(0);
        Ok(Self {
            ledger,
            ledger_id,
            binding_sha256,
        })
    }
}

pub(crate) struct SupportedHostLifecycleCoordinator<'a> {
    authority: BoundAuthority,
    ledger: BoundLedger<'a>,
}

impl<'a> SupportedHostLifecycleCoordinator<'a> {
    pub(in crate::distribution::host_effect) fn bind(
        issuer_id: String,
        ledger_id: String,
        ledger: &'a dyn DurableHostEffectLedger,
    ) -> Result<Self, SupportedHostLifecycleError> {
        let authority = BoundAuthority::generate(issuer_id, ledger_id.clone())?;
        let ledger = BoundLedger::bind(ledger, ledger_id)?;
        let coordinator = Self { authority, ledger };
        coordinator.binding_sha256()?;
        Ok(coordinator)
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::distribution::host_effect) fn accept(
        &self,
        package: PackageIdentity,
        journey: JourneyBinding,
        host: HostCapabilityDeclaration,
        lifecycle: AcceptedLifecyclePlan,
        scope: AcceptedHostScope,
        plan: &HostCommandPlan,
        executable: &PinnedHostExecutable,
        expected_target: ObservedTargetIdentity,
        expected_head: HostEffectLedgerHead,
    ) -> Result<AcceptedHostEffect, SupportedHostLifecycleError> {
        AcceptedHostEffect::accept(
            self.binding_sha256()?,
            package,
            journey,
            host,
            lifecycle,
            scope,
            plan,
            executable,
            expected_target,
            expected_head,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::distribution::host_effect) fn prepare_current(
        &self,
        accepted: &AcceptedHostEffect,
        custody: &mut RootPlanCustody,
        executable: PinnedHostExecutable,
        target: &mut dyn HostTargetObserver,
        clock: &mut dyn RootTrustedClock,
        adapter: &mut dyn DescriptorExecutionAdapter,
    ) -> Result<DescriptorExecutionHandoff, SupportedHostLifecycleError> {
        self.prepare_for_platform(
            DescriptorExecutionPlatform::current(),
            accepted,
            custody,
            executable,
            target,
            clock,
            adapter,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_for_platform(
        &self,
        platform: DescriptorExecutionPlatform,
        accepted: &AcceptedHostEffect,
        custody: &mut RootPlanCustody,
        executable: PinnedHostExecutable,
        target: &mut dyn HostTargetObserver,
        clock: &mut dyn RootTrustedClock,
        adapter: &mut dyn DescriptorExecutionAdapter,
    ) -> Result<DescriptorExecutionHandoff, SupportedHostLifecycleError> {
        // This must remain the first observable decision. In particular,
        // Darwin may not contact clocks, targets, ledgers, permit authority, or
        // descriptor adapters and may not release the plan.
        if platform == DescriptorExecutionPlatform::Darwin {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::UnsupportedPlatform,
            ));
        }
        if !platform.supports_descriptor_execution() {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::DescriptorExecutionUnavailable,
            ));
        }

        accepted.require_coordinator(&self.binding_sha256()?)?;
        accepted.require_plan(custody)?;
        accepted.require_executable(&executable)?;
        let capability = adapter.descriptor_capability()?;
        if capability.platform != platform {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::DescriptorExecutionUnavailable,
            ));
        }

        let time_before = clock.sample()?;
        let mut target_lease = target.acquire(accepted.expected_target())?;
        let target_before = target_lease.identity().clone();
        accepted.require_target(&target_before)?;
        let head = self.ledger.ledger.head().map_err(|_| ledger_rejected())?;
        if &head != accepted.expected_head() {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::StaleLedgerHead,
            ));
        }
        let target_after = target_lease.revalidate()?;
        if target_before != target_after {
            return Err(lifecycle_error(SupportedHostLifecycleErrorId::TargetRace));
        }
        accepted.require_target(&target_after)?;
        let time_after = clock.sample()?;
        time_before.require_successor(&time_after)?;
        let expires_at_unix_ms = time_after
            .unix_ms()
            .checked_add(MAX_PERMIT_TTL_MS)
            .ok_or_else(untrusted_time)?;
        let binding = accepted.derive_binding(time_after.unix_ms(), expires_at_unix_ms, &head)?;
        let (permit, reservation) = self
            .authority
            .authority
            .issue(binding)
            .map_err(|_| authority_rejected())?;
        self.authority
            .authority
            .verify(&permit, time_after.unix_ms())
            .map_err(|_| authority_rejected())?;
        let reserved = self
            .ledger
            .ledger
            .reserve(reservation)
            .map_err(|_| ledger_rejected())?;
        let in_flight = self
            .ledger
            .ledger
            .transition(
                HostEffectTransition::new(
                    permit.permit_id().to_owned(),
                    HostEffectState::Reserved,
                    HostEffectState::InFlight,
                    reserved.current_head().clone(),
                    None,
                )
                .map_err(|_| ledger_rejected())?,
            )
            .map_err(|_| ledger_rejected())?;
        let target_final = target_lease.revalidate()?;
        if target_after != target_final {
            return Err(lifecycle_error(SupportedHostLifecycleErrorId::TargetRace));
        }
        accepted.require_target(&target_final)?;
        // Construct against a non-authoritative plan copy so a final
        // executable revalidation failure cannot consume root plan custody.
        let plan = custody.candidate_plan()?;
        let effect =
            AuthorizedHostEffect::new(permit, in_flight, executable, plan).map_err(|_| {
                lifecycle_error(SupportedHostLifecycleErrorId::HandoffConstructionFailed)
            })?;
        custody.commit_release()?;
        Ok(DescriptorExecutionHandoff {
            capability,
            effect,
            target: target_lease,
        })
    }

    pub(super) fn authorize_recovery(
        &self,
        classification: &PublicationClassification,
        clock: &mut dyn RootTrustedClock,
    ) -> Result<RecoveryAuthorization, SupportedHostLifecycleError> {
        let time = clock.sample()?;
        let head = self.ledger.ledger.head().map_err(|_| ledger_rejected())?;
        issue_recovery_authorization(
            classification,
            &self.binding_sha256()?,
            &head,
            time.unix_ms(),
        )
    }

    pub(super) fn recovery_proposal(
        &self,
        classification: &PublicationClassification,
        authorization: RecoveryAuthorization,
        clock: &mut dyn RootTrustedClock,
    ) -> Result<RecoveryProposal, SupportedHostLifecycleError> {
        let time = clock.sample()?;
        let head = self.ledger.ledger.head().map_err(|_| ledger_rejected())?;
        propose_recovery(
            classification,
            authorization,
            &self.binding_sha256()?,
            &head,
            time.unix_ms(),
        )
    }

    fn binding_sha256(&self) -> Result<String, SupportedHostLifecycleError> {
        if self.authority.ledger_id != self.ledger.ledger_id {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::CoordinatorSubstitution,
            ));
        }
        #[derive(Serialize)]
        struct Binding<'a> {
            schema: &'static str,
            issuer_id: &'a str,
            ledger_id: &'a str,
            authority_binding_sha256: &'a str,
            ledger_binding_sha256: &'a str,
        }
        digest_json(&Binding {
            schema: "harness-ultragoal.supported-host-lifecycle-coordinator.v1",
            issuer_id: &self.authority.issuer_id,
            ledger_id: &self.authority.ledger_id,
            authority_binding_sha256: &self.authority.binding_sha256,
            ledger_binding_sha256: &self.ledger.binding_sha256,
        })
    }

    #[cfg(test)]
    pub(super) fn prepare_as_platform(
        &self,
        platform: DescriptorExecutionPlatform,
        accepted: &AcceptedHostEffect,
        custody: &mut RootPlanCustody,
        executable: PinnedHostExecutable,
        target: &mut dyn HostTargetObserver,
        clock: &mut dyn RootTrustedClock,
        adapter: &mut dyn DescriptorExecutionAdapter,
    ) -> Result<DescriptorExecutionHandoff, SupportedHostLifecycleError> {
        self.prepare_for_platform(
            platform, accepted, custody, executable, target, clock, adapter,
        )
    }
}

pub(crate) struct DescriptorExecutionHandoff {
    capability: DescriptorExecutionCapability,
    effect: AuthorizedHostEffect,
    target: Box<dyn HostTargetLease>,
}

impl std::fmt::Debug for DescriptorExecutionHandoff {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DescriptorExecutionHandoff")
            .field("platform", &self.capability.platform)
            .field("primitive", &self.capability.primitive)
            .finish_non_exhaustive()
    }
}

impl DescriptorExecutionHandoff {
    /// Keeps the target lease owned by the opaque handoff for the complete
    /// synchronous adapter call. Neither effect authority nor the lease can
    /// escape as an owned value.
    pub(in crate::distribution::host_effect) fn with_retained_authority<R>(
        mut self,
        adapter: impl FnOnce(
            &DescriptorExecutionCapability,
            &AuthorizedHostEffect,
            &mut dyn HostTargetLease,
        ) -> R,
    ) -> R {
        adapter(&self.capability, &self.effect, self.target.as_mut())
    }
}

fn recompute_time_attestation(
    sample: &TrustedTimeSample,
) -> Result<String, SupportedHostLifecycleError> {
    #[derive(Serialize)]
    struct Attestation<'a> {
        schema: &'static str,
        source_name: &'a str,
        source_epoch: u64,
        sequence: u64,
        unix_ms: u64,
    }
    digest_json(&Attestation {
        schema: "harness-ultragoal.trusted-time-sample.v1",
        source_name: &sample.source_name,
        source_epoch: sample.source_epoch,
        sequence: sample.sequence,
        unix_ms: sample.unix_ms,
    })
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn digest_json(value: &impl Serialize) -> Result<String, SupportedHostLifecycleError> {
    serde_json::to_vec(value)
        .map(|bytes| digest_bytes(&bytes))
        .map_err(|_| invalid())
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn invalid() -> SupportedHostLifecycleError {
    lifecycle_error(SupportedHostLifecycleErrorId::InvalidAcceptedIdentity)
}

fn untrusted_time() -> SupportedHostLifecycleError {
    lifecycle_error(SupportedHostLifecycleErrorId::UntrustedTime)
}

fn authority_rejected() -> SupportedHostLifecycleError {
    lifecycle_error(SupportedHostLifecycleErrorId::AuthorityRejected)
}

fn ledger_rejected() -> SupportedHostLifecycleError {
    lifecycle_error(SupportedHostLifecycleErrorId::LedgerRejected)
}
