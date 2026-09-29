use super::*;

#[derive(Default)]
pub(crate) struct ReplayState {
    pub(crate) records: BTreeMap<String, LedgerEvent>,
    pub(crate) nonce_owner: BTreeMap<String, String>,
    pub(crate) semantic_owner: BTreeMap<String, String>,
    pub(crate) active_targets: BTreeMap<String, String>,
}

pub(crate) struct LedgerKey(pub(crate) [u8; KEY_BYTES]);

pub(crate) struct ProcessLock(pub(crate) File);

pub(crate) struct EffectOwner<'a> {
    pub(crate) ledger: &'a FileLedger,
    pub(crate) guard: ProcessLock,
    pub(crate) token: ReservationToken,
}

impl Drop for ProcessLock {
    fn drop(&mut self) {
        // SAFETY: the owned file descriptor remains live until this `Drop` implementation returns.
        let _ = unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
    }
}

impl FileLedger {
    pub(crate) fn open_or_initialize(root: &Path, store_id: &str) -> Result<Self, LedgerError> {
        if !valid_digest(store_id) {
            return Err(invalid_store());
        }
        let store = Store::open(root)?;
        store.verify_root()?;
        let lock = match store.exact_stat(LOCK_NAME)? {
            Some(_) => store.open_existing(LOCK_NAME, libc::O_RDWR)?,
            None => {
                let was_empty_before_lock = store.is_empty_unclassified()?;
                if store.exact_stat(LOCK_NAME)?.is_some() {
                    store.open_existing(LOCK_NAME, libc::O_RDWR)?
                } else {
                    if !was_empty_before_lock {
                        return Err(tampered());
                    }
                    store.open_or_create_lock()?
                }
            }
        };
        let _ = exact_identity(&store, LOCK_NAME, &lock, 0o600)?;
        test_before_lock_acquire();
        let guard = ProcessLock::acquire(lock)?;
        store.verify_root()?;
        let locked_identity = exact_identity(&store, LOCK_NAME, &guard.0, 0o600)?;
        let marker = read_bounded(&guard.0, LOCK_MARKER.len() as u64)?;
        let names = store.names()?;
        let fresh = marker.is_empty() && names == BTreeSet::from([LOCK_NAME.to_owned()]);
        if fresh {
            let key = store.create_key()?;
            let key_identity = exact_identity(&store, KEY_NAME, &key, 0o600)?;
            let key = read_key(&key)?;
            let key_id = digest(&key.0);
            let authority_id = authority_id(store_id, &key_id)?;
            let mut lock_writer = &guard.0;
            let lock_identity_before = exact_identity(&store, LOCK_NAME, &guard.0, 0o600)?;
            let initial = initial_payload(
                store_id,
                &authority_id,
                &key_id,
                store.root_identity,
                lock_identity_before,
            )?;
            store.write_initial_state(&encode_snapshot(&initial, &key)?)?;
            lock_writer
                .write_all(LOCK_MARKER)
                .map_err(|_| ledger_io())?;
            guard.0.sync_all().map_err(|_| ledger_io())?;
            store.directory.sync_all().map_err(|_| ledger_io())?;
            let lock_identity = exact_identity(&store, LOCK_NAME, &guard.0, 0o600)?;
            let corrected = initial_payload(
                store_id,
                &authority_id,
                &key_id,
                store.root_identity,
                lock_identity,
            )?;
            store.write_atomic_state(&encode_snapshot(&corrected, &key)?)?;
            return Self::finish_open(store, store_id, key, key_identity, lock_identity);
        }
        if marker != LOCK_MARKER
            || names
                != BTreeSet::from([
                    KEY_NAME.to_owned(),
                    LOCK_NAME.to_owned(),
                    STATE_NAME.to_owned(),
                ])
        {
            return Err(tampered());
        }
        let lock_identity = locked_identity;
        let key_file = store.open_existing(KEY_NAME, libc::O_RDONLY)?;
        let key_identity = exact_identity(&store, KEY_NAME, &key_file, 0o600)?;
        let key = read_key(&key_file)?;
        Self::finish_open(store, store_id, key, key_identity, lock_identity)
    }
    pub(crate) fn open_existing(root: &Path, store_id: &str) -> Result<Self, LedgerError> {
        if !valid_digest(store_id) {
            return Err(invalid_store());
        }
        let store = Store::open(root)?;
        store.verify_root()?;
        match store.exact_stat(LOCK_NAME)? {
            Some(_) => {}
            None if store.is_empty_unclassified()? => return Err(replay_error()),
            None => return Err(tampered()),
        }
        test_before_existing_open();
        let lock = store.open_existing(LOCK_NAME, libc::O_RDWR)?;
        let unlocked_identity = exact_identity(&store, LOCK_NAME, &lock, 0o600)?;
        test_before_lock_acquire();
        let guard = ProcessLock::acquire(lock)?;
        store.verify_root()?;
        let lock_identity = exact_identity(&store, LOCK_NAME, &guard.0, 0o600)?;
        if lock_identity != unlocked_identity
            || read_bounded(&guard.0, LOCK_MARKER.len() as u64)? != LOCK_MARKER
            || store.names()?
                != BTreeSet::from([
                    KEY_NAME.to_owned(),
                    LOCK_NAME.to_owned(),
                    STATE_NAME.to_owned(),
                ])
        {
            return Err(tampered());
        }
        let key_file = store.open_existing(KEY_NAME, libc::O_RDONLY)?;
        let key_identity = exact_identity(&store, KEY_NAME, &key_file, 0o600)?;
        let key = read_key(&key_file)?;
        Self::finish_open(store, store_id, key, key_identity, lock_identity)
    }
    pub(crate) fn finish_open(
        store: Store,
        store_id: &str,
        key: LedgerKey,
        key_identity: FileIdentity,
        lock_identity: FileIdentity,
    ) -> Result<Self, LedgerError> {
        store.validate_complete(key_identity, lock_identity)?;
        let key_id = digest(&key.0);
        let authority_id = authority_id(store_id, &key_id)?;
        let bytes = store.read_state()?;
        let payload = decode_snapshot(
            &bytes,
            &key,
            store_id,
            &authority_id,
            &key_id,
            store.root_identity,
            lock_identity,
        )?;
        replay(&payload)?;
        let ledger = Self {
            store,
            key_identity,
            lock_identity,
            store_id: store_id.to_owned(),
            authority_id,
            local: Mutex::new(ObservedHead {
                generation: payload.generation,
                head_sha256: payload.head_sha256,
            }),
        };
        ledger.verify_store()?;
        Ok(ledger)
    }
    pub(crate) fn authority_id(&self) -> &str {
        &self.authority_id
    }
}
