pub(super) const OUTPUT_LIMIT: usize = 1024 * 1024;

pub struct MarketplaceTransaction {
    candidate_sha256: String,
    previous: Option<Vec<u8>>,
    effect_applied: bool,
}

impl MarketplaceTransaction {
    pub(crate) fn new(
        candidate_sha256: String,
        previous: Option<Vec<u8>>,
        effect_applied: bool,
    ) -> Self {
        Self {
            candidate_sha256,
            previous,
            effect_applied,
        }
    }
}

impl std::fmt::Debug for MarketplaceTransaction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MarketplaceTransaction")
            .field("candidate_sha256", &self.candidate_sha256)
            .field("had_previous", &self.previous.is_some())
            .field("effect_applied", &self.effect_applied)
            .finish()
    }
}

pub fn rollback_marketplace(
    transaction: MarketplaceTransaction,
    effects: &mut impl MarketplaceEffects,
) -> Result<(), DistributionError> {
    if !transaction.effect_applied {
        let observed = effects
            .read(OUTPUT_LIMIT)
            .map_err(|_| error(DistributionErrorId::RollbackFailed))?;
        return if observed == transaction.previous {
            Ok(())
        } else {
            Err(error(DistributionErrorId::ObjectChanged))
        };
    }
    match effects.compare_exchange(
        Some(&transaction.candidate_sha256),
        transaction.previous.as_deref(),
    ) {
        Ok(true) => {}
        Ok(false) => return Err(error(DistributionErrorId::InstallConflict)),
        Err(_) => return Err(error(DistributionErrorId::RollbackFailed)),
    }
    let observed = effects
        .read(OUTPUT_LIMIT)
        .map_err(|_| error(DistributionErrorId::RollbackFailed))?;
    if observed != transaction.previous {
        return Err(error(DistributionErrorId::RollbackFailed));
    }
    Ok(())
}
