//! Darwin production adapters for the accepted migration product protocol.
//!
//! The adapters are crate-private and require a separately preprovisioned,
//! owner-only state root. They do not adopt registry rows or expose a public
//! command. Repository bytes are descriptor-anchored and remain physically
//! untouched; only the fixed host ledger records semantic route state.

mod authority;
mod effects;
mod filesystem;
mod source;
mod store;

#[cfg(test)]
mod test_support;

use self::authority::{DarwinMigrationAuthority, DarwinMigrationAuthorityFactory};
use self::effects::DarwinMigrationEffects;
use self::filesystem::{AnchoredDirectory, FileIdentity, ProcessLock};
use self::source::DarwinMigrationSource;
use self::store::DarwinMigrationStore;
use super::super::digest as digest_bytes;
use super::{ProductMigrationError, ProductMigrationPlan};
use crate::migration::MigrationInventory;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub(crate) use self::authority::DarwinMigrationAuthority as DarwinApplyAuthorizationAuthority;
pub(crate) use self::effects::DarwinMigrationEffects as DarwinConfinedMigrationEffect;
pub(crate) use self::source::DarwinMigrationSource as DarwinMigrationInputSource;
pub(crate) use self::store::DarwinMigrationStore as DarwinDurableMigrationStore;

#[cfg(test)]
pub(crate) use self::test_support::provision_darwin_migration_host_for_test;

const CONFIG_NAME: &str = "authority.json";
const KEY_NAME: &str = "secret.key";
const LOCK_NAME: &str = "migration.lock";
const LEDGER_NAME: &str = "state.json";
const CONFIG_SCHEMA: &str = "DarwinMigrationHostAuthority-v1";
const HOST_SCOPE_DOMAIN: &str = "harness-ultragoal.migration-host-scope.v1";
const BOUNDARY_SOURCE_DOMAIN: &str = "harness-ultragoal.migration-boundary-source.v1";
const MAX_CONFIG_BYTES: u64 = 64 * 1024;
const MAX_KEY_BYTES: u64 = 64;
pub(super) const MAX_HOST_FILE_BYTES: u64 = 8 * 1024 * 1024;
pub(super) const MAX_SOURCE_FILE_BYTES: u64 = 8 * 1024 * 1024;
const KEY_BYTES: usize = 32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HostError {
    code: &'static str,
}

impl HostError {
    pub(crate) const fn new(code: &'static str) -> Self {
        Self { code }
    }

    pub(crate) const fn code(&self) -> &'static str {
        self.code
    }

    fn product(self) -> ProductMigrationError {
        ProductMigrationError::new(self.code)
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code)
    }
}

impl std::error::Error for HostError {}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HostAuthorityConfig {
    schema_version: String,
    repository_scope_sha256: String,
    candidate_id: String,
    product_version: String,
    principal_id: String,
    authorization_authority_id: String,
    compatibility_boundary_authority_id: String,
    compatibility_boundary_source_identity_sha256: String,
    clock_anchor_unix_ms: u64,
    clock_anchor_monotonic_ns: u64,
    config_sha256: String,
}

impl HostAuthorityConfig {
    #[allow(clippy::too_many_arguments)]
    fn issue(
        repository_scope_sha256: String,
        candidate_id: String,
        product_version: String,
        clock_anchor_unix_ms: u64,
        clock_anchor_monotonic_ns: u64,
    ) -> Result<Self, HostError> {
        let principal_id = "migration-host-operator".to_owned();
        let authorization_authority_id = "migration-host-authorization-authority".to_owned();
        let compatibility_boundary_authority_id =
            "migration-host-compatibility-boundary-authority".to_owned();
        let compatibility_boundary_source_identity_sha256 = digest_bytes(
            format!(
                "{BOUNDARY_SOURCE_DOMAIN}|{repository_scope_sha256}|{candidate_id}|{product_version}"
            )
            .as_bytes(),
        );
        let config_sha256 = config_digest(
            &repository_scope_sha256,
            &candidate_id,
            &product_version,
            &principal_id,
            &authorization_authority_id,
            &compatibility_boundary_authority_id,
            &compatibility_boundary_source_identity_sha256,
            clock_anchor_unix_ms,
            clock_anchor_monotonic_ns,
        );
        let value = Self {
            schema_version: CONFIG_SCHEMA.to_owned(),
            repository_scope_sha256,
            candidate_id,
            product_version,
            principal_id,
            authorization_authority_id,
            compatibility_boundary_authority_id,
            compatibility_boundary_source_identity_sha256,
            clock_anchor_unix_ms,
            clock_anchor_monotonic_ns,
            config_sha256,
        };
        value.validate()?;
        Ok(value)
    }

    fn validate(&self) -> Result<(), HostError> {
        if self.schema_version != CONFIG_SCHEMA
            || !super::super::valid_sha256(&self.repository_scope_sha256)
            || !super::super::valid_sha256(&self.candidate_id)
            || !valid_product_version(&self.product_version)
            || !super::super::valid_identifier(&self.principal_id)
            || !super::super::valid_identifier(&self.authorization_authority_id)
            || !super::super::valid_identifier(&self.compatibility_boundary_authority_id)
            || !super::super::valid_sha256(&self.compatibility_boundary_source_identity_sha256)
            || self.clock_anchor_unix_ms == 0
            || self.clock_anchor_monotonic_ns == 0
            || self.config_sha256
                != config_digest(
                    &self.repository_scope_sha256,
                    &self.candidate_id,
                    &self.product_version,
                    &self.principal_id,
                    &self.authorization_authority_id,
                    &self.compatibility_boundary_authority_id,
                    &self.compatibility_boundary_source_identity_sha256,
                    self.clock_anchor_unix_ms,
                    self.clock_anchor_monotonic_ns,
                )
        {
            return Err(HostError::new("migration-host-authority-config-invalid"));
        }
        Ok(())
    }

    fn canonical_bytes(&self) -> Result<Vec<u8>, HostError> {
        serde_json::to_vec(self)
            .map_err(|_| HostError::new("migration-host-authority-config-invalid"))
    }
}

pub(super) struct HostContext {
    repository: AnchoredDirectory,
    state: AnchoredDirectory,
    config: HostAuthorityConfig,
    config_bytes: Vec<u8>,
    config_identity: FileIdentity,
    key: [u8; KEY_BYTES],
    key_bytes: Vec<u8>,
    key_identity: FileIdentity,
    lock_identity: FileIdentity,
    _process_lock: ProcessLock,
    io: Mutex<()>,
    #[cfg(test)]
    trusted_time_advance_ms: std::sync::atomic::AtomicU64,
    #[cfg(test)]
    fail_on_cas: Mutex<Option<usize>>,
    #[cfg(test)]
    cas_count: std::sync::atomic::AtomicUsize,
}

impl HostContext {
    fn open(
        repository_root: &Path,
        state_root: &Path,
        expected_candidate_id: &str,
        expected_product_version: &str,
    ) -> Result<Arc<Self>, HostError> {
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (
                repository_root,
                state_root,
                expected_candidate_id,
                expected_product_version,
            );
            return Err(HostError::new("migration-host-darwin-required"));
        }
        #[cfg(target_vendor = "apple")]
        {
            let repository = AnchoredDirectory::open_absolute(repository_root, false)?;
            let state = AnchoredDirectory::open_absolute(state_root, true)?;
            if state.path().starts_with(repository.path())
                || repository.path().starts_with(state.path())
            {
                return Err(HostError::new("migration-host-root-overlap-refused"));
            }
            let expected_scope = repository_scope_id(
                repository.path(),
                repository.identity(),
                expected_candidate_id,
            );
            let config_file = state.read_regular(CONFIG_NAME, true, MAX_CONFIG_BYTES)?;
            let config: HostAuthorityConfig = serde_json::from_slice(&config_file.bytes)
                .map_err(|_| HostError::new("migration-host-authority-config-invalid"))?;
            config.validate()?;
            if config.canonical_bytes()? != config_file.bytes
                || config.repository_scope_sha256 != expected_scope
                || config.candidate_id != expected_candidate_id
                || config.product_version != expected_product_version
            {
                return Err(HostError::new("migration-host-authority-binding-refused"));
            }
            let key_file = state.read_regular(KEY_NAME, true, MAX_KEY_BYTES)?;
            if key_file.bytes.len() != KEY_BYTES {
                return Err(HostError::new("migration-host-secret-key-invalid"));
            }
            let mut key = [0_u8; KEY_BYTES];
            key.copy_from_slice(&key_file.bytes);
            let lock_identity = state.fixed_file_identity(LOCK_NAME, 0)?;
            if lock_identity.size != 0 {
                return Err(HostError::new("migration-host-lock-file-refused"));
            }
            let _ = state.fixed_file_identity(LEDGER_NAME, MAX_HOST_FILE_BYTES)?;
            let process_lock = ProcessLock::acquire(&state, LOCK_NAME)?;
            let value = Arc::new(Self {
                repository,
                state,
                config,
                config_bytes: config_file.bytes,
                config_identity: config_file.identity,
                key,
                key_bytes: key_file.bytes,
                key_identity: key_file.identity,
                lock_identity,
                _process_lock: process_lock,
                io: Mutex::new(()),
                #[cfg(test)]
                trusted_time_advance_ms: std::sync::atomic::AtomicU64::new(0),
                #[cfg(test)]
                fail_on_cas: Mutex::new(None),
                #[cfg(test)]
                cas_count: std::sync::atomic::AtomicUsize::new(0),
            });
            value.verify_static()?;
            let _ = value.trusted_time()?;
            Ok(value)
        }
    }

    pub(super) fn verify_static(&self) -> Result<(), HostError> {
        self.repository.verify()?;
        self.state.verify()?;
        self.state.verify_fixed_file(
            CONFIG_NAME,
            self.config_identity,
            &self.config_bytes,
            MAX_CONFIG_BYTES,
        )?;
        self.state.verify_fixed_file(
            KEY_NAME,
            self.key_identity,
            &self.key_bytes,
            MAX_KEY_BYTES,
        )?;
        if self.state.fixed_file_identity(LOCK_NAME, 0)? != self.lock_identity {
            return Err(HostError::new("migration-host-lock-substituted"));
        }
        self._process_lock.verify(&self.state, LOCK_NAME)?;
        let ledger = self
            .state
            .fixed_file_identity(LEDGER_NAME, MAX_HOST_FILE_BYTES)?;
        if ledger.size == 0 {
            return Err(HostError::new("migration-host-state-empty"));
        }
        Ok(())
    }

    pub(super) fn trusted_time(&self) -> Result<TrustedTime, HostError> {
        self.verify_static_without_clock()?;
        let monotonic_ns = monotonic_nanoseconds()?;
        #[cfg(test)]
        let monotonic_ns = self
            .trusted_time_advance_ms
            .load(std::sync::atomic::Ordering::SeqCst)
            .checked_mul(1_000_000)
            .and_then(|advance| monotonic_ns.checked_add(advance))
            .ok_or_else(|| HostError::new("migration-host-clock-invalid"))?;
        if monotonic_ns < self.config.clock_anchor_monotonic_ns {
            return Err(HostError::new("migration-host-clock-rollback"));
        }
        let delta_ms = monotonic_ns
            .checked_sub(self.config.clock_anchor_monotonic_ns)
            .ok_or_else(|| HostError::new("migration-host-clock-rollback"))?
            / 1_000_000;
        let unix_ms = self
            .config
            .clock_anchor_unix_ms
            .checked_add(delta_ms)
            .ok_or_else(|| HostError::new("migration-host-clock-invalid"))?;
        Ok(TrustedTime {
            monotonic_ns,
            unix_ms,
        })
    }

    fn verify_static_without_clock(&self) -> Result<(), HostError> {
        self.repository.verify()?;
        self.state.verify()?;
        self.state.verify_fixed_file(
            CONFIG_NAME,
            self.config_identity,
            &self.config_bytes,
            MAX_CONFIG_BYTES,
        )?;
        self.state.verify_fixed_file(
            KEY_NAME,
            self.key_identity,
            &self.key_bytes,
            MAX_KEY_BYTES,
        )?;
        self._process_lock.verify(&self.state, LOCK_NAME)
    }

    fn state(&self) -> &AnchoredDirectory {
        &self.state
    }

    fn repository(&self) -> &AnchoredDirectory {
        &self.repository
    }

    pub(super) fn scope_id(&self) -> &str {
        &self.config.repository_scope_sha256
    }

    pub(super) fn config(&self) -> &HostAuthorityConfig {
        &self.config
    }

    pub(super) fn key(&self) -> &[u8; KEY_BYTES] {
        &self.key
    }

    pub(super) fn io(&self) -> &Mutex<()> {
        &self.io
    }

    #[cfg(test)]
    pub(super) fn advance_time_for_test(&self, milliseconds: u64) {
        self.trusted_time_advance_ms
            .fetch_add(milliseconds, std::sync::atomic::Ordering::SeqCst);
    }
}

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

#[allow(clippy::too_many_arguments)]
fn config_digest(
    repository_scope_sha256: &str,
    candidate_id: &str,
    product_version: &str,
    principal_id: &str,
    authorization_authority_id: &str,
    compatibility_boundary_authority_id: &str,
    compatibility_boundary_source_identity_sha256: &str,
    clock_anchor_unix_ms: u64,
    clock_anchor_monotonic_ns: u64,
) -> String {
    digest_bytes(
        format!(
            "{CONFIG_SCHEMA}|{repository_scope_sha256}|{candidate_id}|{product_version}|{principal_id}|{authorization_authority_id}|{compatibility_boundary_authority_id}|{compatibility_boundary_source_identity_sha256}|{clock_anchor_unix_ms}|{clock_anchor_monotonic_ns}"
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
