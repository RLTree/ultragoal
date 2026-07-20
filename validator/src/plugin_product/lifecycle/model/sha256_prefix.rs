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

pub(super) type LifecycleActionAuthority = Arc<AtomicU8>;
pub(super) type RecoveryStateAuthority = Arc<Mutex<Option<LifecycleState>>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(super) enum LifecycleActionState {
    Planned = 0,
    Transferred = 1,
    Applying = 2,
    RecoveryAvailable = 3,
    Recovering = 4,
    Closed = 5,
}

impl LifecycleActionState {
    fn decode(value: u8) -> Result<Self, LifecycleError> {
        match value {
            0 => Ok(Self::Planned),
            1 => Ok(Self::Transferred),
            2 => Ok(Self::Applying),
            3 => Ok(Self::RecoveryAvailable),
            4 => Ok(Self::Recovering),
            5 => Ok(Self::Closed),
            _ => Err(LifecycleError::InvalidTransition),
        }
    }
}

#[derive(Clone, Default)]
pub(super) enum PlanAuthorizationSeal {
    #[default]
    Unsealed,
    Sealed(Box<PlanAuthorizationSealData>),
}

#[derive(Clone)]
pub(super) struct PlanAuthorizationSealData {
    issuance_id: u64,
    observed: LifecycleState,
    request: LifecycleRequest,
    action_state: LifecycleActionAuthority,
    recovery_state: RecoveryStateAuthority,
}
