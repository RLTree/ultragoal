use super::*;

impl FileLedger {
    pub(in crate::routine_work::runtime_adapter::production::custody) fn open_existing(
        root: &Path,
    ) -> Result<(Self, LocalHead), RoutineError> {
        let store = Store::open(root)?;
        let names = store.names()?;
        if names
            != BTreeSet::from([
                KEY_NAME.to_owned(),
                LOCK_NAME.to_owned(),
                STATE_NAME.to_owned(),
            ])
        {
            return Err(error("routine-production-authority-store-incomplete"));
        }
        let lock = store.open_existing(LOCK_NAME, libc::O_RDWR)?;
        let lock_identity = store.exact_identity(LOCK_NAME, &lock, 0o600)?;
        Self::load_complete(store, ProcessLock::acquire(lock)?, lock_identity)
    }

    pub(in crate::routine_work::runtime_adapter::production::custody::store) fn open_or_initialize(
        root: &Path,
    ) -> Result<(Self, LocalHead), RoutineError> {
        let store = Store::open(root)?;
        let names = store.names()?;
        if names
            == BTreeSet::from([
                KEY_NAME.to_owned(),
                LOCK_NAME.to_owned(),
                STATE_NAME.to_owned(),
            ])
        {
            let lock = store.open_existing(LOCK_NAME, libc::O_RDWR)?;
            let lock_identity = store.exact_identity(LOCK_NAME, &lock, 0o600)?;
            return Self::load_complete(store, ProcessLock::acquire(lock)?, lock_identity);
        }
        if !names.is_empty() {
            return Err(error("routine-production-authority-store-incomplete"));
        }
        let guard = store.acquire_initial_lock()?;
        let lock_identity = store.exact_identity(LOCK_NAME, &guard.0, 0o600)?;
        let names = store.names()?;
        if names == BTreeSet::from([LOCK_NAME.to_owned()]) {
            let key = store.create_key()?;
            let key_identity = store.exact_identity(KEY_NAME, &key, 0o600)?;
            let key = read_key(&key)?;
            let key_id = sha256(key.bytes());
            let authority_id = authority_id(&key_id, store.identity, lock_identity)?;
            let payload = initial_payload(&authority_id, &key_id, store.identity, lock_identity)?;
            store.write_initial_state(&encode(&payload, &key)?)?;
            store.validate_complete(key_identity, lock_identity)?;
        } else {
            return Err(error("routine-production-authority-store-incomplete"));
        }
        Self::load_complete(store, guard, lock_identity)
    }
    pub(super) fn load_complete(
        store: Store,
        guard: ProcessLock,
        lock_identity: FileIdentity,
    ) -> Result<(Self, LocalHead), RoutineError> {
        let key_file = store.open_existing(KEY_NAME, libc::O_RDONLY)?;
        let key_identity = store.exact_identity(KEY_NAME, &key_file, 0o600)?;
        let key = read_key(&key_file)?;
        let key_id = sha256(key.bytes());
        let authority_id = authority_id(&key_id, store.identity, lock_identity)?;
        let bytes = store.read_state()?;
        let payload = decode(
            &bytes,
            &key,
            &authority_id,
            &key_id,
            store.identity,
            lock_identity,
        )?;
        validate_payload(&payload)?;
        let head_sha256 = sha256(&bytes);
        let state_identity = store.state_identity()?;
        store.validate_complete(key_identity, lock_identity)?;
        drop(guard);
        Ok((
            Self {
                store,
                key_identity,
                lock_identity,
                key_id,
                authority_id,
            },
            LocalHead {
                generation: payload.generation,
                head_sha256,
                state_identity,
            },
        ))
    }
}
