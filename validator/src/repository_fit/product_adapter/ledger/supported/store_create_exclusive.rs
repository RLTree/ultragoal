use super::*;

impl Store {
    pub(crate) fn create_exclusive(&self, name: &str, mode: u32) -> Result<File, LedgerError> {
        validate_name(name)?;
        let name = CString::new(name).map_err(|_| invalid_store())?;
        let descriptor = unsafe {
            libc::openat(
                self.directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
                mode,
            )
        };
        if descriptor < 0 {
            return Err(match std::io::Error::last_os_error().raw_os_error() {
                Some(libc::EEXIST) => replay_error(),
                _ => ledger_io(),
            });
        }
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }
    pub(crate) fn open_existing(&self, name: &str, flags: i32) -> Result<File, LedgerError> {
        validate_name(name)?;
        let name = CString::new(name).map_err(|_| invalid_store())?;
        let descriptor = unsafe {
            libc::openat(
                self.directory.as_raw_fd(),
                name.as_ptr(),
                flags | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            )
        };
        if descriptor < 0 {
            return Err(tampered());
        }
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }
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
            if identity.links != 1
                || identity.uid != unsafe { libc::geteuid() }
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
    pub(crate) fn exact_stat(&self, name: &str) -> Result<Option<FileIdentity>, LedgerError> {
        let name = CString::new(name).map_err(|_| invalid_store())?;
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        let result = unsafe {
            libc::fstatat(
                self.directory.as_raw_fd(),
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result != 0 {
            return match std::io::Error::last_os_error().raw_os_error() {
                Some(libc::ENOENT) => Ok(None),
                _ => Err(ledger_io()),
            };
        }
        let stat = unsafe { stat.assume_init() };
        Ok(Some(stat_identity(&stat)))
    }
}

impl ProcessLock {
    pub(crate) fn acquire(file: File) -> Result<Self, LedgerError> {
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return Err(ledger_io());
        }
        Ok(Self(file))
    }
}

pub(crate) fn initial_payload(
    store_id: &str,
    authority_id: &str,
    key_id: &str,
    root_identity: RootIdentity,
    lock_identity: FileIdentity,
) -> Result<SnapshotPayload, LedgerError> {
    let head_sha256 = digest(
        &serde_json::to_vec(&(
            INITIAL_HEAD_DOMAIN,
            store_id,
            authority_id,
            key_id,
            root_identity,
            lock_identity,
        ))
        .map_err(|_| invalid_transition())?,
    );
    Ok(SnapshotPayload {
        schema_version: LEDGER_SCHEMA.to_owned(),
        store_id: store_id.to_owned(),
        authority_id: authority_id.to_owned(),
        key_id: key_id.to_owned(),
        root_identity,
        lock_identity,
        generation: 0,
        head_sha256,
        events: Vec::new(),
    })
}

pub(crate) fn authority_id(store_id: &str, key_id: &str) -> Result<String, LedgerError> {
    serde_json::to_vec(&(AUTHORITY_DOMAIN, store_id, key_id))
        .map(|bytes| digest(&bytes))
        .map_err(|_| invalid_transition())
}
