use super::filesystem::AnchoredDirectory;
use super::store::HostLedger;
use super::{
    CONFIG_NAME, HostAuthorityConfig, HostError, KEY_BYTES, KEY_NAME, LEDGER_NAME, LOCK_NAME,
    monotonic_nanoseconds, realtime_milliseconds, repository_scope_id,
};
use crate::migration::MigrationInventory;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;

pub(crate) fn provision_darwin_migration_host_for_test(
    repository_root: &Path,
    state_root: &Path,
    inventory: &MigrationInventory,
    product_version: &str,
) -> Result<(), HostError> {
    #[cfg(not(target_vendor = "apple"))]
    {
        let _ = (repository_root, state_root, inventory, product_version);
        return Err(HostError::new("migration-host-darwin-required"));
    }
    #[cfg(target_vendor = "apple")]
    {
        if state_root.exists() {
            return Err(HostError::new("migration-host-state-already-exists"));
        }
        fs::create_dir(state_root)
            .map_err(|_| HostError::new("migration-host-test-provision-failed"))?;
        fs::set_permissions(state_root, fs::Permissions::from_mode(0o700))
            .map_err(|_| HostError::new("migration-host-test-provision-failed"))?;
        let canonical_state = fs::canonicalize(state_root)
            .map_err(|_| HostError::new("migration-host-test-provision-failed"))?;
        if canonical_state != state_root {
            return Err(HostError::new("migration-host-root-not-canonical"));
        }
        let repository = AnchoredDirectory::open_absolute(repository_root, false)?;
        let scope = repository_scope_id(
            repository.path(),
            repository.identity(),
            inventory.candidate_id(),
        );
        let config = HostAuthorityConfig::issue(
            scope.clone(),
            inventory.candidate_id().to_owned(),
            product_version.to_owned(),
            realtime_milliseconds()?,
            monotonic_nanoseconds()?,
        )?;
        let mut key = [0_u8; KEY_BYTES];
        getrandom::fill(&mut key)
            .map_err(|_| HostError::new("migration-host-random-unavailable"))?;
        let ledger = HostLedger::empty(&scope)?;
        write_fixed(
            &canonical_state.join(CONFIG_NAME),
            &config.canonical_bytes()?,
        )?;
        write_fixed(&canonical_state.join(KEY_NAME), &key)?;
        write_fixed(&canonical_state.join(LOCK_NAME), &[])?;
        write_fixed(
            &canonical_state.join(LEDGER_NAME),
            &ledger.canonical_bytes()?,
        )?;
        let directory = File::open(&canonical_state)
            .map_err(|_| HostError::new("migration-host-test-provision-failed"))?;
        directory
            .sync_all()
            .map_err(|_| HostError::new("migration-host-test-provision-failed"))?;
        Ok(())
    }
}

fn write_fixed(path: &Path, bytes: &[u8]) -> Result<(), HostError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|_| HostError::new("migration-host-test-provision-failed"))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| HostError::new("migration-host-test-provision-failed"))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|_| HostError::new("migration-host-test-provision-failed"))
}
