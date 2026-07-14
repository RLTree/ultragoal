#[derive(Clone, Copy)]
pub(super) struct TrustedTime {
    pub(super) monotonic_ns: u64,
    pub(super) unix_ms: u64,
}

pub(crate) struct DarwinMigrationHost;

pub(crate) struct DarwinMigrationAdapters {
    pub(crate) source: DarwinMigrationSource,
    pub(crate) store: DarwinMigrationStore,
    pub(crate) effects: DarwinMigrationEffects,
    authority_factory: DarwinMigrationAuthorityFactory,
    context: Arc<HostContext>,
}

impl DarwinMigrationHost {
    pub(crate) fn open(
        repository_root: &Path,
        state_root: &Path,
        inventory: MigrationInventory,
        expected_candidate_id: &str,
        expected_product_version: &str,
    ) -> Result<DarwinMigrationAdapters, HostError> {
        if inventory.candidate_id() != expected_candidate_id {
            return Err(HostError::new("migration-host-inventory-candidate-refused"));
        }
        let context = HostContext::open(
            repository_root,
            state_root,
            expected_candidate_id,
            expected_product_version,
        )?;
        let source = DarwinMigrationSource::new(context.clone(), inventory)?;
        let store = DarwinMigrationStore::new(context.clone())?;
        let effects = DarwinMigrationEffects::new(context.clone());
        let authority_factory = DarwinMigrationAuthorityFactory::new(context.clone());
        Ok(DarwinMigrationAdapters {
            source,
            store,
            effects,
            authority_factory,
            context,
        })
    }
}

impl DarwinMigrationAdapters {
    pub(crate) fn boundary_authority(&self) -> Result<DarwinMigrationAuthority, HostError> {
        self.authority_factory.boundary_authority()
    }

    pub(crate) fn apply_authority(
        &self,
        plan: &ProductMigrationPlan,
    ) -> Result<DarwinMigrationAuthority, HostError> {
        self.authority_factory.apply_authority(plan)
    }

    pub(crate) fn recovery_authority(
        &self,
        operation_id: &str,
        plan: &ProductMigrationPlan,
    ) -> Result<DarwinMigrationAuthority, HostError> {
        let operation = super::DurableMigrationStore::load_operation(&self.store, operation_id)
            .map_err(|_| HostError::new("migration-host-recovery-operation-load-failed"))?
            .ok_or_else(|| HostError::new("migration-host-recovery-operation-unknown"))?;
        self.authority_factory.recovery_authority(&operation, plan)
    }

    #[cfg(test)]
    pub(crate) fn advance_trusted_time_for_test(&self, milliseconds: u64) {
        self.context.advance_time_for_test(milliseconds);
    }
}

fn repository_scope_id(path: &Path, identity: FileIdentity, candidate_id: &str) -> String {
    digest_bytes(
        format!(
            "{HOST_SCOPE_DOMAIN}|{}|{}|{}|{}",
            path.display(),
            identity.device,
            identity.inode,
            candidate_id
        )
        .as_bytes(),
    )
}

fn config_digest(config: &HostAuthorityConfig) -> String {
    digest_bytes(
        format!(
            "{CONFIG_SCHEMA}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            config.repository_scope_sha256,
            config.candidate_id,
            config.product_version,
            config.principal_id,
            config.authorization_authority_id,
            config.compatibility_boundary_authority_id,
            config.compatibility_boundary_source_identity_sha256,
            config.clock_anchor_unix_ms,
            config.clock_anchor_monotonic_ns,
        )
        .as_bytes(),
    )
}

fn valid_product_version(value: &str) -> bool {
    let components = value.split('.').collect::<Vec<_>>();
    components.len() == 3
        && components.iter().all(|component| {
            !component.is_empty()
                && (component.len() == 1 || !component.starts_with('0'))
                && component.bytes().all(|byte| byte.is_ascii_digit())
                && component.parse::<u32>().is_ok()
        })
}

pub(super) fn monotonic_nanoseconds() -> Result<u64, HostError> {
    let mut value = std::mem::MaybeUninit::<libc::timespec>::uninit();
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, value.as_mut_ptr()) } != 0 {
        return Err(HostError::new("migration-host-clock-unavailable"));
    }
    let value = unsafe { value.assume_init() };
    if value.tv_sec < 0 || !(0..1_000_000_000).contains(&value.tv_nsec) {
        return Err(HostError::new("migration-host-clock-invalid"));
    }
    let seconds =
        u64::try_from(value.tv_sec).map_err(|_| HostError::new("migration-host-clock-invalid"))?;
    let nanos =
        u64::try_from(value.tv_nsec).map_err(|_| HostError::new("migration-host-clock-invalid"))?;
    seconds
        .checked_mul(1_000_000_000)
        .and_then(|value| value.checked_add(nanos))
        .ok_or_else(|| HostError::new("migration-host-clock-invalid"))
}

#[cfg(test)]
pub(super) fn realtime_milliseconds() -> Result<u64, HostError> {
    let mut value = std::mem::MaybeUninit::<libc::timespec>::uninit();
    if unsafe { libc::clock_gettime(libc::CLOCK_REALTIME, value.as_mut_ptr()) } != 0 {
        return Err(HostError::new("migration-host-clock-unavailable"));
    }
    let value = unsafe { value.assume_init() };
    if value.tv_sec < 0 || !(0..1_000_000_000).contains(&value.tv_nsec) {
        return Err(HostError::new("migration-host-clock-invalid"));
    }
    let seconds =
        u64::try_from(value.tv_sec).map_err(|_| HostError::new("migration-host-clock-invalid"))?;
    let nanos =
        u64::try_from(value.tv_nsec).map_err(|_| HostError::new("migration-host-clock-invalid"))?;
    seconds
        .checked_mul(1_000)
        .and_then(|value| value.checked_add(nanos / 1_000_000))
        .ok_or_else(|| HostError::new("migration-host-clock-invalid"))
}
