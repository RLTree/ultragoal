use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::marketplace::MarketplacePlan;
use crate::distribution::package::PackageSnapshot;
use crate::distribution::reader::{sha256, validate_relative_path};
use crate::distribution::spec::digest;
use serde::Serialize;

const INSTALL_LIMIT: usize = 65 * 1024 * 1024;

impl std::fmt::Debug for MarketplacePlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MarketplacePlan")
            .field("expected_sha256", &self.expected_sha256())
            .field("replacement_byte_length", &self.replacement().len())
            .field("package", self.package())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallScope {
    Repository,
    Personal,
    RepositoryFixture,
    PersonalFixture,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExpectedPrior {
    Absent,
    ExactDigest(String),
}

#[derive(Clone, Debug)]
pub struct InstallPlan {
    context_id: String,
    candidate_id: String,
    scope: InstallScope,
    target: String,
    package_sha256: String,
    expected_prior: ExpectedPrior,
}

impl InstallPlan {
    pub fn new(
        context_id: String,
        candidate_id: String,
        scope: InstallScope,
        target: String,
        package_sha256: String,
        expected_prior: ExpectedPrior,
    ) -> Result<Self, DistributionError> {
        validate_relative_path(&target)?;
        if !digest(&context_id)
            || !digest(&candidate_id)
            || !digest(&package_sha256)
            || matches!(&expected_prior, ExpectedPrior::ExactDigest(value) if !digest(value))
            || matches!(scope, InstallScope::Repository | InstallScope::Personal)
                && target != "plugins/harness-ultragoal.hugpkg"
        {
            return Err(error(DistributionErrorId::InvalidSpec));
        }
        Ok(Self {
            context_id,
            candidate_id,
            scope,
            target,
            package_sha256,
            expected_prior,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct InstallSnapshot {
    context_id: String,
    candidate_id: String,
    scope: InstallScope,
    target_id: String,
    package_sha256: String,
    replaced_existing: bool,
}

impl InstallSnapshot {
    pub fn context_id(&self) -> &str {
        &self.context_id
    }

    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub const fn scope(&self) -> InstallScope {
        self.scope
    }

    pub fn package_sha256(&self) -> &str {
        &self.package_sha256
    }

    pub const fn replaced_existing(&self) -> bool {
        self.replaced_existing
    }
}

pub struct InstallTransaction {
    snapshot: InstallSnapshot,
    target: String,
    previous: Option<Vec<u8>>,
}

impl std::fmt::Debug for InstallTransaction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InstallTransaction")
            .field("snapshot", &self.snapshot)
            .field("target_id", &sha256(self.target.as_bytes()))
            .field("had_previous", &self.previous.is_some())
            .finish()
    }
}

impl InstallTransaction {
    pub fn snapshot(&self) -> &InstallSnapshot {
        &self.snapshot
    }
}

pub trait InstallEffects {
    fn read_installed(&mut self, target: &str, maximum: usize) -> Result<Option<Vec<u8>>, ()>;

    /// Atomically compares the current destination with `expected` and, only
    /// when it matches, replaces it with `replacement` (`None` removes it).
    ///
    /// `Ok(true)` means the comparison and transition occurred as one
    /// indivisible effect. `Ok(false)` means the destination did not match and
    /// MUST remain unchanged. Implementations that cannot provide that
    /// contract must return `Err(())`; callers must not emulate it with a
    /// separate read followed by an unconditional write.
    fn compare_exchange_installed(
        &mut self,
        target: &str,
        expected: &ExpectedPrior,
        replacement: Option<&[u8]>,
    ) -> Result<bool, ()>;
}

pub fn install(
    plan: &InstallPlan,
    package: &PackageSnapshot,
    effects: &mut impl InstallEffects,
) -> Result<InstallTransaction, DistributionError> {
    if plan.context_id != package.context_id()
        || plan.candidate_id != package.candidate_id()
        || plan.package_sha256 != package.package_sha256()
    {
        return Err(error(DistributionErrorId::ArchiveMismatch));
    }
    let previous = read(effects, &plan.target)?;
    if !prior_matches(&plan.expected_prior, previous.as_deref()) {
        return Err(error(DistributionErrorId::InstallConflict));
    }
    match effects.compare_exchange_installed(
        &plan.target,
        &plan.expected_prior,
        Some(package.archive()),
    ) {
        Ok(true) => {}
        Ok(false) => return Err(error(DistributionErrorId::InstallConflict)),
        Err(()) => return Err(error(DistributionErrorId::EffectFailed)),
    }
    let verification = match read(effects, &plan.target) {
        Ok(Some(bytes)) if sha256(&bytes) == plan.package_sha256 => Ok(()),
        Ok(_) => Err(error(DistributionErrorId::ArchiveMismatch)),
        Err(failure) => Err(failure),
    };
    if let Err(failure) = verification {
        restore_if_candidate(
            effects,
            &plan.target,
            &plan.package_sha256,
            previous.as_deref(),
        )?;
        return Err(failure);
    }
    Ok(InstallTransaction {
        snapshot: InstallSnapshot {
            context_id: plan.context_id.clone(),
            candidate_id: plan.candidate_id.clone(),
            scope: plan.scope,
            target_id: sha256(plan.target.as_bytes()),
            package_sha256: plan.package_sha256.clone(),
            replaced_existing: previous.is_some(),
        },
        target: plan.target.clone(),
        previous,
    })
}

pub fn uninstall(
    target: &str,
    snapshot: &InstallSnapshot,
    effects: &mut impl InstallEffects,
) -> Result<(), DistributionError> {
    validate_relative_path(target)?;
    if sha256(target.as_bytes()) != snapshot.target_id {
        return Err(error(DistributionErrorId::InstallConflict));
    }
    let expected = ExpectedPrior::ExactDigest(snapshot.package_sha256.clone());
    match effects.compare_exchange_installed(target, &expected, None) {
        Ok(true) => {}
        Ok(false) => return Err(error(DistributionErrorId::InstallConflict)),
        Err(()) => return Err(error(DistributionErrorId::EffectFailed)),
    }
    if let Some(current) = read(effects, target)? {
        return Err(error(if prior_matches(&expected, Some(&current)) {
            DistributionErrorId::EffectFailed
        } else {
            DistributionErrorId::InstallConflict
        }));
    }
    Ok(())
}

pub fn rollback_install(
    transaction: InstallTransaction,
    effects: &mut impl InstallEffects,
) -> Result<(), DistributionError> {
    restore_if_candidate(
        effects,
        &transaction.target,
        &transaction.snapshot.package_sha256,
        transaction.previous.as_deref(),
    )
}

fn read(
    effects: &mut impl InstallEffects,
    target: &str,
) -> Result<Option<Vec<u8>>, DistributionError> {
    let bytes = effects
        .read_installed(target, INSTALL_LIMIT)
        .map_err(|_| error(DistributionErrorId::EffectFailed))?;
    if bytes
        .as_ref()
        .is_some_and(|value| value.len() > INSTALL_LIMIT)
    {
        return Err(error(DistributionErrorId::ObjectTooLarge));
    }
    Ok(bytes)
}

fn prior_matches(expected: &ExpectedPrior, actual: Option<&[u8]>) -> bool {
    match expected {
        ExpectedPrior::Absent => actual.is_none(),
        ExpectedPrior::ExactDigest(expected) => {
            actual.is_some_and(|bytes| sha256(bytes) == *expected)
        }
    }
}

fn restore_if_candidate(
    effects: &mut impl InstallEffects,
    target: &str,
    installed_sha256: &str,
    previous: Option<&[u8]>,
) -> Result<(), DistributionError> {
    let expected = ExpectedPrior::ExactDigest(installed_sha256.to_owned());
    match effects.compare_exchange_installed(target, &expected, previous) {
        Ok(true) => {}
        Ok(false) => return Err(error(DistributionErrorId::InstallConflict)),
        Err(()) => return Err(error(DistributionErrorId::RollbackFailed)),
    }
    let restored = read(effects, target).map_err(|_| error(DistributionErrorId::RollbackFailed))?;
    if restored.as_deref() != previous {
        return Err(error(DistributionErrorId::RollbackFailed));
    }
    Ok(())
}
