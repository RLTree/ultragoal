use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct RepositoryFitStoredRootIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) uid: u32,
    pub(crate) gid: u32,
    pub(crate) mode: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct RepositoryFitStoredLockIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) links: u64,
    pub(crate) uid: u32,
    pub(crate) gid: u32,
    pub(crate) mode: u32,
    pub(crate) length: u64,
    pub(crate) changed_seconds: i64,
    pub(crate) changed_nanoseconds: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct RepositoryFitAuthorityDeviceDrift {
    pub(crate) authority_inventory_sha256: String,
    pub(crate) ledger_sha256: String,
    pub(crate) authority_id: String,
    pub(crate) generation: u64,
    pub(crate) head_sha256: String,
    pub(crate) event_count: usize,
    pub(crate) reservation_count: usize,
    pub(crate) nonterminal_reservation_count: usize,
    pub(crate) stored_root: RepositoryFitStoredRootIdentity,
    pub(crate) current_root: RepositoryFitStoredRootIdentity,
    pub(crate) stored_lock: RepositoryFitStoredLockIdentity,
    pub(crate) current_lock: RepositoryFitStoredLockIdentity,
}

impl FileRepositoryFitLedger {
    pub(crate) fn assess_device_drift(
        root: &std::path::Path,
        store_id: &str,
    ) -> Result<Option<RepositoryFitAuthorityDeviceDrift>, LedgerError> {
        #[cfg(target_vendor = "apple")]
        {
            supported::assess_device_drift(root, store_id)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (root, store_id);
            Err(LedgerError::new(LedgerErrorId::UnsupportedHost))
        }
    }
}

#[cfg(all(test, target_vendor = "apple"))]
pub(crate) fn simulate_device_drift_for_test(
    root: &std::path::Path,
    replacement_device: u64,
) -> Result<(), LedgerError> {
    supported::simulate_device_drift_for_test(root, replacement_device)
}
