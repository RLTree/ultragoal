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
        matches!(self, Self::Darwin | Self::Linux | Self::FreeBsd)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum DescriptorExecutionPrimitive {
    DarwinPosixSpawnSuspendedLoadedVnode,
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
                DescriptorExecutionPlatform::Darwin,
                DescriptorExecutionPrimitive::DarwinPosixSpawnSuspendedLoadedVnode
            ) | (
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
