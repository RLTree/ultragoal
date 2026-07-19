use super::*;

impl Store {
    pub(crate) fn read_state(&self) -> Result<Vec<u8>, LedgerError> {
        let file = self.open_existing(STATE_NAME, libc::O_RDONLY)?;
        let _ = exact_identity(self, STATE_NAME, &file, 0o600)?;
        read_bounded(&file, MAX_LEDGER_BYTES)
    }

    pub(crate) fn write_initial_state(&self, bytes: &[u8]) -> Result<(), LedgerError> {
        let mut file = self.create_exclusive(STATE_NAME, 0o600)?;
        file.write_all(bytes).map_err(|_| ledger_io())?;
        file.sync_all().map_err(|_| ledger_io())?;
        self.directory.sync_all().map_err(|_| ledger_io())?;
        let _ = exact_identity(self, STATE_NAME, &file, 0o600)?;
        Ok(())
    }

    pub(crate) fn write_atomic_state(&self, bytes: &[u8]) -> Result<(), LedgerError> {
        if bytes.is_empty() || bytes.len() as u64 > MAX_LEDGER_BYTES {
            return Err(invalid_transition());
        }
        let temporary = temporary_name()?;
        let mut file = self.create_exclusive(&temporary, 0o600)?;
        let result = (|| {
            file.write_all(bytes).map_err(|_| ledger_io())?;
            file.sync_all().map_err(|_| ledger_io())?;
            let identity = file_identity(&file.metadata().map_err(|_| ledger_io())?);
            // SAFETY: `geteuid` reads the calling process's effective UID and has no preconditions.
            let expected_uid = unsafe { libc::geteuid() };
            if identity.links != 1
                || identity.uid != expected_uid
                || identity.mode & 0o7777 != 0o600
                || identity.length != bytes.len() as u64
            {
                return Err(tampered());
            }
            test_before_atomic_publish();
            rename_relative(&self.directory, &temporary, STATE_NAME)?;
            self.directory.sync_all().map_err(|_| ledger_io())?;
            let state = self.open_existing(STATE_NAME, libc::O_RDONLY)?;
            let _ = exact_identity(self, STATE_NAME, &state, 0o600)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = unlink_relative(&self.directory, &temporary);
        }
        result
    }

    pub(crate) fn validate_complete(
        &self,
        key_identity: FileIdentity,
        lock_identity: FileIdentity,
    ) -> Result<(), LedgerError> {
        self.verify_root()?;
        if self.names()?
            != BTreeSet::from([
                KEY_NAME.to_owned(),
                LOCK_NAME.to_owned(),
                STATE_NAME.to_owned(),
            ])
        {
            return Err(tampered());
        }
        let key = self.open_existing(KEY_NAME, libc::O_RDONLY)?;
        let lock = self.open_existing(LOCK_NAME, libc::O_RDONLY)?;
        let state = self.open_existing(STATE_NAME, libc::O_RDONLY)?;
        if exact_identity(self, KEY_NAME, &key, 0o600)? != key_identity
            || exact_identity(self, LOCK_NAME, &lock, 0o600)? != lock_identity
            || read_bounded(&lock, LOCK_MARKER.len() as u64)? != LOCK_MARKER
        {
            return Err(tampered());
        }
        let _ = exact_identity(self, STATE_NAME, &state, 0o600)?;
        self.verify_root()
    }
}
