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

pub trait InstallEffects {
    fn read_installed(&mut self, target: &str, maximum: usize) -> Result<Option<Vec<u8>>, ()>;

    fn installed_postimage(
        &mut self,
        _target: &str,
        _maximum: usize,
    ) -> Result<Option<InstalledPostimage>, ()> {
        Err(())
    }

    fn current_install_authority(
        &mut self,
        _snapshot: &InstallSnapshot,
        _binding: &crate::distribution::host_capability::JourneyBinding,
    ) -> Result<CurrentInstallAuthority, ()> {
        Err(())
    }

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
            target: plan.target.clone(),
            package_sha256: plan.package_sha256.clone(),
            replaced_existing: previous.is_some(),
            postimage: effects
                .installed_postimage(&plan.target, INSTALL_LIMIT)
                .ok()
                .flatten()
                .filter(|row| row.object_sha256 == plan.package_sha256),
            journey_binding_sha256: None,
        },
        target: plan.target.clone(),
        previous,
    })
}
