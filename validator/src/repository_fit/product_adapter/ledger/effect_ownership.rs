use super::*;

impl EffectOwner<'_> {
    pub(crate) fn terminal(
        self,
        state: RepositoryFitLedgerState,
        terminal_sha256: &str,
        error_id: Option<AdapterErrorId>,
        tick: u64,
    ) -> Result<(), LedgerError> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.terminal(state, terminal_sha256, error_id, tick)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (self, state, terminal_sha256, error_id, tick);
            Err(LedgerError::new(LedgerErrorId::UnsupportedHost))
        }
    }
}

#[cfg(all(test, target_vendor = "apple"))]
pub(crate) fn before_lock_acquire_for_test(action: impl FnOnce() + 'static) {
    supported::before_lock_acquire_for_test(action);
}

#[cfg(all(test, target_vendor = "apple"))]
pub(crate) fn before_atomic_publish_for_test(action: impl FnOnce() + 'static) {
    supported::before_atomic_publish_for_test(action);
}

#[cfg(all(test, target_vendor = "apple"))]
pub(crate) fn before_existing_open_for_test(action: impl FnOnce() + 'static) {
    supported::before_existing_open_for_test(action);
}
