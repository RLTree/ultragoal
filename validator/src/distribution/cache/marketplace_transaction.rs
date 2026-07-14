const MARKETPLACE_LIMIT: usize = 1024 * 1024;

pub trait MarketplaceEffects {
    fn read(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()>;
    fn compare_exchange(
        &mut self,
        expected_sha256: Option<&str>,
        replacement: Option<&[u8]>,
    ) -> Result<bool, ()>;
}

pub fn apply_marketplace(
    plan: &MarketplacePlan,
    effects: &mut impl MarketplaceEffects,
) -> Result<MarketplaceTransaction, DistributionError> {
    let before = read_marketplace(effects)?;
    if before.as_deref().map(sha256).as_deref() != plan.expected_sha256() {
        return Err(error(DistributionErrorId::InstallConflict));
    }
    match effects.compare_exchange(plan.expected_sha256(), Some(plan.replacement())) {
        Ok(true) => {}
        Ok(false) => return Err(error(DistributionErrorId::InstallConflict)),
        Err(()) => return Err(error(DistributionErrorId::EffectFailed)),
    }
    let after = match read_marketplace(effects) {
        Ok(value) => value,
        Err(failure) => {
            restore_marketplace(plan, effects)?;
            return Err(failure);
        }
    };
    if after.as_deref() != Some(plan.replacement()) {
        restore_marketplace(plan, effects)?;
        return Err(error(DistributionErrorId::ArchiveMismatch));
    }
    Ok(MarketplaceTransaction::new(
        sha256(plan.replacement()),
        before,
    ))
}

fn read_marketplace(
    effects: &mut impl MarketplaceEffects,
) -> Result<Option<Vec<u8>>, DistributionError> {
    let value = effects
        .read(MARKETPLACE_LIMIT)
        .map_err(|_| error(DistributionErrorId::EffectFailed))?;
    if value
        .as_ref()
        .is_some_and(|bytes| bytes.len() > MARKETPLACE_LIMIT)
    {
        return Err(error(DistributionErrorId::ObjectTooLarge));
    }
    Ok(value)
}

fn restore_marketplace(
    plan: &MarketplacePlan,
    effects: &mut impl MarketplaceEffects,
) -> Result<(), DistributionError> {
    let candidate = sha256(plan.replacement());
    match effects.compare_exchange(Some(&candidate), plan.rollback.as_deref()) {
        Ok(true) => Ok(()),
        Ok(false) => Err(error(DistributionErrorId::InstallConflict)),
        Err(()) => Err(error(DistributionErrorId::RollbackFailed)),
    }
}
