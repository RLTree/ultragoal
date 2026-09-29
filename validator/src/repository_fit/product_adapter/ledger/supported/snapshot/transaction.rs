use super::*;

impl FileLedger {
    pub(crate) fn with_snapshot<T>(
        &self,
        operation: impl FnOnce(&mut SnapshotPayload, &ReplayState) -> Result<(T, bool), LedgerError>,
    ) -> Result<T, LedgerError> {
        let guard = self.acquire_process_lock()?;
        self.with_held_snapshot(&guard, operation)
    }

    pub(crate) fn with_held_snapshot<T>(
        &self,
        guard: &ProcessLock,
        operation: impl FnOnce(&mut SnapshotPayload, &ReplayState) -> Result<(T, bool), LedgerError>,
    ) -> Result<T, LedgerError> {
        if exact_identity(&self.store, LOCK_NAME, &guard.0, 0o600)? != self.lock_identity {
            return Err(tampered());
        }
        let mut local = self.local.lock().map_err(|_| ledger_io())?;
        self.verify_store()?;
        let key_file = self.store.open_existing(KEY_NAME, libc::O_RDONLY)?;
        if exact_identity(&self.store, KEY_NAME, &key_file, 0o600)? != self.key_identity {
            return Err(tampered());
        }
        let key = read_key(&key_file)?;
        let key_id = digest(&key.0);
        let bytes = self.store.read_state()?;
        let mut payload = decode_snapshot(
            &bytes,
            &key,
            &self.store_id,
            &self.authority_id,
            &key_id,
            self.store.root_identity,
            self.lock_identity,
        )?;
        if payload.generation < local.generation
            || (payload.generation == local.generation && payload.head_sha256 != local.head_sha256)
        {
            return Err(tampered());
        }
        let replayed = replay(&payload)?;
        let (value, changed) = operation(&mut payload, &replayed)?;
        if changed {
            self.store
                .write_atomic_state(&encode_snapshot(&payload, &key)?)?;
            let confirmed = decode_snapshot(
                &self.store.read_state()?,
                &key,
                &self.store_id,
                &self.authority_id,
                &key_id,
                self.store.root_identity,
                self.lock_identity,
            )?;
            replay(&confirmed)?;
            if confirmed != payload {
                return Err(tampered());
            }
        }
        self.store
            .validate_complete(self.key_identity, self.lock_identity)?;
        local.generation = payload.generation;
        local.head_sha256.clone_from(&payload.head_sha256);
        self.verify_store()?;
        Ok(value)
    }
    pub(crate) fn verify_store(&self) -> Result<(), LedgerError> {
        self.store.verify_root()?;
        self.store
            .validate_complete(self.key_identity, self.lock_identity)
    }
    #[cfg(test)]
    pub(crate) fn snapshot_for_test(&self) -> Result<Vec<u8>, LedgerError> {
        self.verify_store()?;
        self.store.read_state()
    }
}
