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
        Err(_) => return Err(error(DistributionErrorId::EffectFailed)),
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
    effects: &mut ScopedInstall,
) -> Result<(), RollbackInstallError> {
    if let Err(failure) = transaction.revalidate_for_rollback(effects) {
        return Err(RollbackInstallError::refused(failure, transaction));
    }
    let expected = match transaction.snapshot.postimage().cloned() {
        Some(expected) => expected,
        None => {
            return Err(RollbackInstallError::refused(
                error(DistributionErrorId::ProvenanceMismatch),
                transaction,
            ));
        }
    };
    match effects.compare_exchange_installed_postimage(
        &transaction.target,
        &expected,
        transaction.previous.as_deref(),
    ) {
        Ok(true) => {}
        Ok(false) => {
            return Err(RollbackInstallError::refused(
                error(DistributionErrorId::InstallConflict),
                transaction,
            ));
        }
        Err(_) => {
            return Err(RollbackInstallError::refused(
                error(DistributionErrorId::RollbackFailed),
                transaction,
            ));
        }
    }
    let restored = read(effects, &transaction.target)
        .map_err(|_| RollbackInstallError::committed(error(DistributionErrorId::RollbackFailed)))?;
    if restored.as_deref() != transaction.previous.as_deref() {
        return Err(RollbackInstallError::committed(error(
            DistributionErrorId::RollbackFailed,
        )));
    }
    Ok(())
}

#[derive(Debug)]
pub enum RollbackInstallError {
    Refused {
        error: DistributionError,
        transaction: Box<InstallTransaction>,
    },
    Committed {
        error: DistributionError,
    },
}

impl RollbackInstallError {
    fn refused(error: DistributionError, transaction: InstallTransaction) -> Self {
        Self::Refused {
            error,
            transaction: Box::new(transaction),
        }
    }

    fn committed(error: DistributionError) -> Self {
        Self::Committed { error }
    }

    pub const fn id(&self) -> DistributionErrorId {
        match self {
            Self::Refused { error, .. } | Self::Committed { error } => error.id(),
        }
    }

    pub fn into_transaction(self) -> Option<InstallTransaction> {
        match self {
            Self::Refused { transaction, .. } => Some(*transaction),
            Self::Committed { .. } => None,
        }
    }
}

impl From<RollbackInstallError> for DistributionError {
    fn from(value: RollbackInstallError) -> Self {
        match value {
            RollbackInstallError::Refused { error, .. }
            | RollbackInstallError::Committed { error } => error,
        }
    }
}

pub(super) fn restore_if_candidate(
    effects: &mut impl InstallEffects,
    target: &str,
    installed_sha256: &str,
    previous: Option<&[u8]>,
) -> Result<(), DistributionError> {
    restore_if_digest_candidate(effects, target, installed_sha256, previous)
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

fn restore_if_digest_candidate(
    effects: &mut impl InstallEffects,
    target: &str,
    installed_sha256: &str,
    previous: Option<&[u8]>,
) -> Result<(), DistributionError> {
    let expected = ExpectedPrior::ExactDigest(installed_sha256.to_owned());
    match effects.compare_exchange_installed(target, &expected, previous) {
        Ok(true) => {}
        Ok(false) => return Err(error(DistributionErrorId::InstallConflict)),
        Err(_) => return Err(error(DistributionErrorId::RollbackFailed)),
    }
    let restored = read(effects, target).map_err(|_| error(DistributionErrorId::RollbackFailed))?;
    if restored.as_deref() != previous {
        return Err(error(DistributionErrorId::RollbackFailed));
    }
    Ok(())
}
