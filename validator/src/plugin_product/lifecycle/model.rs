use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU8, AtomicU64, Ordering as AtomicOrdering},
};

const SHA256_PREFIX: &str = "sha256:";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleIntent {
    FreshInstall,
    MonotonicUpdate,
    FailedUpdateRecovery,
    AuthorizedRollback,
    IdempotentReinstall,
    UninstallTeardown,
    StaleCacheRecovery,
    RepeatUse,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleEffect {
    InstallPackage,
    RefreshCache,
    RestorePriorAuthority,
    VerifyInstalledBytes,
    RemoveInstalledPackage,
    RemoveCache,
    VerifyTeardown,
    ProbeRuntime,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    pub fn parse(value: &str) -> Result<Self, LifecycleError> {
        let mut parts = value.split('.');
        let major = number(parts.next())?;
        let minor = number(parts.next())?;
        let patch = number(parts.next())?;
        if parts.next().is_some() {
            return Err(LifecycleError::InvalidVersion);
        }
        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn number(value: Option<&str>) -> Result<u64, LifecycleError> {
    let value = value.ok_or(LifecycleError::InvalidVersion)?;
    if value.is_empty() || (value.len() > 1 && value.starts_with('0')) {
        return Err(LifecycleError::InvalidVersion);
    }
    value.parse().map_err(|_| LifecycleError::InvalidVersion)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageAuthority {
    pub version: Version,
    pub package_sha256: String,
    pub inventory_sha256: String,
    pub candidate_id: String,
}

impl PackageAuthority {
    pub fn validate(&self) -> Result<(), LifecycleError> {
        for digest in [
            &self.package_sha256,
            &self.inventory_sha256,
            &self.candidate_id,
        ] {
            validate_digest(digest)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleState {
    pub installed: Option<PackageAuthority>,
    pub cache: Option<PackageAuthority>,
    pub generation: u64,
    pub recovery_required: bool,
}

impl LifecycleState {
    pub fn validate(&self) -> Result<(), LifecycleError> {
        if let Some(installed) = &self.installed {
            installed.validate()?;
        }
        if let Some(cache) = &self.cache {
            cache.validate()?;
        }
        if self.installed.is_none() && self.cache.is_some() && !self.recovery_required {
            return Err(LifecycleError::InconsistentState);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleAuthorization {
    pub allow_host_write: bool,
    pub allow_downgrade: bool,
    pub expected_installed_sha256: Option<String>,
}

impl LifecycleAuthorization {
    pub(super) fn validate(&self) -> Result<(), LifecycleError> {
        if let Some(digest) = &self.expected_installed_sha256 {
            validate_digest(digest)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleRequest {
    pub intent: LifecycleIntent,
    pub target: Option<PackageAuthority>,
    pub prior_authority: Option<LifecycleState>,
    pub authorization: LifecycleAuthorization,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecyclePlan {
    pub schema_version: String,
    pub plan_id: String,
    pub authorization_sha256: String,
    pub intent: LifecycleIntent,
    pub before: LifecycleState,
    pub expected_after: LifecycleState,
    pub effects: Vec<LifecycleEffect>,
    pub rollback_state: LifecycleState,
    pub writes_host_state: bool,
    #[serde(skip, default)]
    pub(super) authorization_seal: PlanAuthorizationSeal,
}

static NEXT_PLAN_ISSUANCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(super) enum LifecycleActionState {
    Planned = 0,
    Applying = 1,
    RecoveryAvailable = 2,
    Recovering = 3,
    Closed = 4,
}

impl LifecycleActionState {
    fn decode(value: u8) -> Result<Self, LifecycleError> {
        match value {
            0 => Ok(Self::Planned),
            1 => Ok(Self::Applying),
            2 => Ok(Self::RecoveryAvailable),
            3 => Ok(Self::Recovering),
            4 => Ok(Self::Closed),
            _ => Err(LifecycleError::InvalidTransition),
        }
    }
}

#[derive(Clone)]
pub(super) enum PlanAuthorizationSeal {
    Unsealed,
    Sealed {
        issuance_id: u64,
        observed: LifecycleState,
        request: LifecycleRequest,
        action_state: Arc<AtomicU8>,
        recovery_state: Arc<Mutex<Option<LifecycleState>>>,
    },
}

impl PlanAuthorizationSeal {
    pub(super) fn issue(observed: &LifecycleState, request: &LifecycleRequest) -> Self {
        Self::Sealed {
            issuance_id: NEXT_PLAN_ISSUANCE.fetch_add(1, AtomicOrdering::Relaxed),
            observed: observed.clone(),
            request: request.clone(),
            action_state: Arc::new(AtomicU8::new(LifecycleActionState::Planned as u8)),
            recovery_state: Arc::new(Mutex::new(None)),
        }
    }

    pub(super) fn authorized_context(&self) -> Option<(&LifecycleState, &LifecycleRequest)> {
        match self {
            Self::Unsealed => None,
            Self::Sealed {
                observed, request, ..
            } => Some((observed, request)),
        }
    }

    pub(super) fn consume(&self) -> Result<(), LifecycleError> {
        let Self::Sealed { action_state, .. } = self else {
            return Err(LifecycleError::UnsealedPlan);
        };
        action_state
            .compare_exchange(
                LifecycleActionState::Planned as u8,
                LifecycleActionState::Applying as u8,
                AtomicOrdering::AcqRel,
                AtomicOrdering::Acquire,
            )
            .map(|_| ())
            .map_err(|_| LifecycleError::ReplayedPlan)
    }

    pub(super) fn finish_apply(
        &self,
        recovery_state: Option<&LifecycleState>,
    ) -> Result<(), LifecycleError> {
        let Self::Sealed {
            action_state,
            recovery_state: authorized_recovery_state,
            ..
        } = self
        else {
            return Err(LifecycleError::UnsealedPlan);
        };
        if let Some(state) = recovery_state {
            state.validate()?;
        }
        let mut gate = authorized_recovery_state
            .lock()
            .map_err(|_| LifecycleError::InvalidTransition)?;
        *gate = recovery_state.cloned();
        let next = if recovery_state.is_some() {
            LifecycleActionState::RecoveryAvailable
        } else {
            LifecycleActionState::Closed
        };
        action_state
            .compare_exchange(
                LifecycleActionState::Applying as u8,
                next as u8,
                AtomicOrdering::AcqRel,
                AtomicOrdering::Acquire,
            )
            .map(|_| ())
            .map_err(|_| LifecycleError::InvalidTransition)
    }

    pub(super) fn recovery_gate(
        &self,
    ) -> Result<(Arc<AtomicU8>, Arc<Mutex<Option<LifecycleState>>>), LifecycleError> {
        let Self::Sealed {
            action_state,
            recovery_state,
            ..
        } = self
        else {
            return Err(LifecycleError::UnsealedPlan);
        };
        Ok((Arc::clone(action_state), Arc::clone(recovery_state)))
    }
}

impl Default for PlanAuthorizationSeal {
    fn default() -> Self {
        Self::Unsealed
    }
}

impl fmt::Debug for PlanAuthorizationSeal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsealed => formatter.write_str("Unsealed"),
            Self::Sealed {
                issuance_id,
                observed,
                request,
                ..
            } => formatter
                .debug_struct("Sealed")
                .field("issuance_id", issuance_id)
                .field("observed", observed)
                .field("request", request)
                .finish_non_exhaustive(),
        }
    }
}

impl PartialEq for PlanAuthorizationSeal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unsealed, Self::Unsealed) => true,
            (
                Self::Sealed {
                    issuance_id: left_id,
                    observed: left_observed,
                    request: left_request,
                    ..
                },
                Self::Sealed {
                    issuance_id: right_id,
                    observed: right_observed,
                    request: right_request,
                    ..
                },
            ) => {
                left_id == right_id
                    && left_observed == right_observed
                    && left_request == right_request
            }
            _ => false,
        }
    }
}

impl Eq for PlanAuthorizationSeal {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyDisposition {
    Applied,
    RecoveredAfterFailure,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyReport {
    pub plan_id: String,
    pub disposition: ApplyDisposition,
    pub state: LifecycleState,
    pub completed_effects: Vec<LifecycleEffect>,
    pub failed_effect: Option<LifecycleEffect>,
    pub causal_error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryToken {
    pub schema_version: String,
    pub plan_id: String,
    pub prior: LifecycleState,
    pub expected_current: LifecycleState,
    #[serde(skip, default)]
    pub(super) authorization_seal: RecoveryAuthorizationSeal,
}

#[derive(Clone)]
pub(super) enum RecoveryAuthorizationSeal {
    Unsealed,
    Sealed {
        issuance_id: u64,
        plan_id: String,
        prior: LifecycleState,
        expected_current: LifecycleState,
        action_state: Arc<AtomicU8>,
        recovery_state: Arc<Mutex<Option<LifecycleState>>>,
    },
}

impl RecoveryAuthorizationSeal {
    pub(super) fn issue(
        plan_id: &str,
        prior: &LifecycleState,
        expected_current: &LifecycleState,
        action_state: Arc<AtomicU8>,
        recovery_state: Arc<Mutex<Option<LifecycleState>>>,
    ) -> Self {
        Self::Sealed {
            issuance_id: NEXT_PLAN_ISSUANCE.fetch_add(1, AtomicOrdering::Relaxed),
            plan_id: plan_id.to_owned(),
            prior: prior.clone(),
            expected_current: expected_current.clone(),
            action_state,
            recovery_state,
        }
    }

    pub(super) fn authorized_restore(&self) -> Option<(&str, &LifecycleState, &LifecycleState)> {
        match self {
            Self::Unsealed => None,
            Self::Sealed {
                plan_id,
                prior,
                expected_current,
                ..
            } => Some((plan_id, prior, expected_current)),
        }
    }

    pub(super) fn consume_for_observed(
        &self,
        observed_state: &LifecycleState,
    ) -> Result<(), LifecycleError> {
        let Self::Sealed {
            action_state,
            recovery_state,
            ..
        } = self
        else {
            return Err(LifecycleError::UnsealedRecoveryToken);
        };
        let recovery_state = recovery_state
            .lock()
            .map_err(|_| LifecycleError::InvalidTransition)?;
        loop {
            let action_state_value =
                LifecycleActionState::decode(action_state.load(AtomicOrdering::Acquire))?;
            match action_state_value {
                LifecycleActionState::Planned | LifecycleActionState::Applying => {
                    return Err(LifecycleError::RecoveryUnavailable);
                }
                LifecycleActionState::RecoveryAvailable => {
                    if recovery_state.as_ref() != Some(observed_state) {
                        return Err(LifecycleError::StaleRecoveryToken);
                    }
                    if action_state
                        .compare_exchange_weak(
                            LifecycleActionState::RecoveryAvailable as u8,
                            LifecycleActionState::Recovering as u8,
                            AtomicOrdering::AcqRel,
                            AtomicOrdering::Acquire,
                        )
                        .is_ok()
                    {
                        return Ok(());
                    }
                }
                LifecycleActionState::Recovering | LifecycleActionState::Closed => {
                    return Err(LifecycleError::ReplayedRecoveryToken);
                }
            }
        }
    }

    pub(super) fn close(&self) -> Result<(), LifecycleError> {
        let Self::Sealed { action_state, .. } = self else {
            return Err(LifecycleError::UnsealedRecoveryToken);
        };
        action_state
            .compare_exchange(
                LifecycleActionState::Recovering as u8,
                LifecycleActionState::Closed as u8,
                AtomicOrdering::AcqRel,
                AtomicOrdering::Acquire,
            )
            .map(|_| ())
            .map_err(|_| LifecycleError::InvalidTransition)
    }
}

impl Default for RecoveryAuthorizationSeal {
    fn default() -> Self {
        Self::Unsealed
    }
}

impl fmt::Debug for RecoveryAuthorizationSeal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsealed => formatter.write_str("Unsealed"),
            Self::Sealed {
                issuance_id,
                plan_id,
                prior,
                expected_current,
                ..
            } => formatter
                .debug_struct("Sealed")
                .field("issuance_id", issuance_id)
                .field("plan_id", plan_id)
                .field("prior", prior)
                .field("expected_current", expected_current)
                .finish_non_exhaustive(),
        }
    }
}

impl PartialEq for RecoveryAuthorizationSeal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unsealed, Self::Unsealed) => true,
            (
                Self::Sealed {
                    issuance_id: left_id,
                    plan_id: left_plan,
                    prior: left_prior,
                    expected_current: left_current,
                    ..
                },
                Self::Sealed {
                    issuance_id: right_id,
                    plan_id: right_plan,
                    prior: right_prior,
                    expected_current: right_current,
                    ..
                },
            ) => {
                left_id == right_id
                    && left_plan == right_plan
                    && left_prior == right_prior
                    && left_current == right_current
            }
            _ => false,
        }
    }
}

impl Eq for RecoveryAuthorizationSeal {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LifecycleError {
    AuthorizationRequired,
    DowngradeAuthorizationRequired,
    ExpectedPriorMismatch,
    InconsistentState,
    InvalidDigest,
    InvalidTransition,
    InvalidVersion,
    MissingPriorAuthority,
    ReadEffectFailed {
        effect: LifecycleEffect,
        causal_error: String,
    },
    RecoveryObservationFailed(String),
    RecoveryFailed(String),
    RecoveryStateMismatch,
    RecoveryUnavailable,
    ReplayedPlan,
    ReplayedRecoveryToken,
    StalePlan,
    StaleRecoveryToken,
    UnsealedPlan,
    UnsealedRecoveryToken,
    VerificationFailed,
}

pub trait LifecycleEffectAdapter {
    fn execute(
        &mut self,
        effect: LifecycleEffect,
        expected_after: &LifecycleState,
    ) -> Result<(), String>;
    fn restore(&mut self, prior: &LifecycleState) -> Result<(), String>;
    fn observe_state(&self) -> Result<LifecycleState, String>;
}

pub(super) fn validate_digest(value: &str) -> Result<(), LifecycleError> {
    let Some(hex) = value.strip_prefix(SHA256_PREFIX) else {
        return Err(LifecycleError::InvalidDigest);
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(LifecycleError::InvalidDigest);
    }
    Ok(())
}
