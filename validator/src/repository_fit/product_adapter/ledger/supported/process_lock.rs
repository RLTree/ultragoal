use super::*;

impl ProcessLock {
    pub(crate) fn acquire(file: File) -> Result<Self, LedgerError> {
        // SAFETY: `file` owns a live descriptor that remains open in the returned lock.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return Err(ledger_io());
        }
        Ok(Self(file))
    }
}

impl FileLedger {
    pub(crate) fn acquire_process_lock(&self) -> Result<ProcessLock, LedgerError> {
        self.store.verify_root()?;
        let lock = self.store.open_existing(LOCK_NAME, libc::O_RDWR)?;
        if exact_identity(&self.store, LOCK_NAME, &lock, 0o600)? != self.lock_identity {
            return Err(tampered());
        }
        test_before_lock_acquire();
        let guard = ProcessLock::acquire(lock)?;
        if exact_identity(&self.store, LOCK_NAME, &guard.0, 0o600)? != self.lock_identity {
            return Err(tampered());
        }
        self.verify_store()?;
        Ok(guard)
    }
}
