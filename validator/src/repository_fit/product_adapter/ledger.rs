//! Durable repository-fit production reservation and terminal ledger.
//!
//! This deliberately owns repository-fit records instead of reusing the
//! distribution host-effect authority model. It borrows the accepted storage
//! properties: descriptor binding, owner-only objects, a process-shared lock,
//! an authenticated hash chain, atomic publication, and directory fsync.

use serde::{Deserialize, Serialize};

use super::{adapter_error, AdapterErrorId, FitAdapterError};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum RepositoryFitLedgerState {
    Reserved,
    Committed,
    RolledBack,
    Rejected,
    Interrupted,
    Ambiguous,
}

impl RepositoryFitLedgerState {
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Reserved => "reserved",
            Self::Committed => "committed",
            Self::RolledBack => "rolled_back",
            Self::Rejected => "rejected",
            Self::Interrupted => "interrupted",
            Self::Ambiguous => "ambiguous",
        }
    }

    const fn terminal(self) -> bool {
        !matches!(self, Self::Reserved)
    }
}

pub(super) struct ReservationRequest<'a> {
    pub(super) binding_sha256: &'a str,
    pub(super) semantic_effect_id: &'a str,
    pub(super) target_scope_id: &'a str,
    pub(super) permit_id: &'a str,
    pub(super) nonce_sha256: &'a str,
    pub(super) issued_tick: u64,
    pub(super) expires_tick: u64,
}

pub(super) struct ReservationToken {
    reservation_id: String,
    binding_sha256: String,
    semantic_effect_id: String,
    target_scope_id: String,
    permit_id: String,
    nonce_sha256: String,
}

impl std::fmt::Debug for ReservationToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReservationToken")
            .field("reservation", &"[bound]")
            .field("binding", &"[bound]")
            .field("semantic_effect", &"[bound]")
            .field("target_scope", &"[bound]")
            .field("permit", &"[bound]")
            .field("nonce", &"[redacted]")
            .finish()
    }
}

pub(super) struct ExistingReservation {
    token: ReservationToken,
    state: RepositoryFitLedgerState,
    terminal_sha256: Option<String>,
}

impl ExistingReservation {
    pub(super) const fn state(&self) -> RepositoryFitLedgerState {
        self.state
    }

    pub(super) fn token(self) -> ReservationToken {
        self.token
    }

    #[cfg(test)]
    pub(super) fn terminal_sha256(&self) -> Option<&str> {
        self.terminal_sha256.as_deref()
    }
}

pub(super) enum ReservationDecision {
    Acquired(ReservationToken),
    Existing(ExistingReservation),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LedgerErrorId {
    UnsupportedHost,
    InvalidStore,
    Tampered,
    Replay,
    ActiveLease,
    InvalidTransition,
    Io,
}

#[derive(Debug)]
pub(super) struct LedgerError {
    id: LedgerErrorId,
}

impl LedgerError {
    const fn new(id: LedgerErrorId) -> Self {
        Self { id }
    }

    pub(super) const fn id(&self) -> LedgerErrorId {
        self.id
    }

    pub(super) const fn authority_invariant() -> Self {
        Self::new(LedgerErrorId::Tampered)
    }

    pub(super) const fn adapter_error(&self) -> FitAdapterError {
        let id = match self.id {
            LedgerErrorId::UnsupportedHost => AdapterErrorId::UnsupportedHost,
            LedgerErrorId::InvalidStore => AdapterErrorId::ApplyPermitInvalid,
            LedgerErrorId::Replay => AdapterErrorId::ApplyPermitReplayed,
            LedgerErrorId::ActiveLease => AdapterErrorId::ApplyLeaseInvalid,
            LedgerErrorId::Tampered | LedgerErrorId::InvalidTransition | LedgerErrorId::Io => {
                AdapterErrorId::ApplyOutcomeInvalid
            }
        };
        adapter_error(id)
    }
}

pub(super) struct FileRepositoryFitLedger {
    #[cfg(target_vendor = "apple")]
    inner: supported::FileLedger,
}

impl FileRepositoryFitLedger {
    pub(super) fn open_or_initialize(
        root: &std::path::Path,
        store_id: &str,
    ) -> Result<Self, LedgerError> {
        #[cfg(target_vendor = "apple")]
        {
            supported::FileLedger::open_or_initialize(root, store_id).map(|inner| Self { inner })
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (root, store_id);
            Err(LedgerError::new(LedgerErrorId::UnsupportedHost))
        }
    }

    pub(super) fn authority_id(&self) -> &str {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.authority_id()
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            unreachable!("unsupported hosts cannot construct a ledger")
        }
    }

    pub(super) fn reserve(
        &self,
        request: ReservationRequest<'_>,
    ) -> Result<ReservationDecision, LedgerError> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.reserve(request)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = request;
            Err(LedgerError::new(LedgerErrorId::UnsupportedHost))
        }
    }

    pub(super) fn terminal(
        &self,
        token: ReservationToken,
        state: RepositoryFitLedgerState,
        terminal_sha256: &str,
        error_id: Option<AdapterErrorId>,
        tick: u64,
    ) -> Result<(), LedgerError> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner
                .terminal(token, state, terminal_sha256, error_id, tick)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (token, state, terminal_sha256, error_id, tick);
            Err(LedgerError::new(LedgerErrorId::UnsupportedHost))
        }
    }

    #[cfg(test)]
    pub(super) fn snapshot_for_test(&self) -> Result<Vec<u8>, LedgerError> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.snapshot_for_test()
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            Err(LedgerError::new(LedgerErrorId::UnsupportedHost))
        }
    }
}

#[cfg(target_vendor = "apple")]
mod supported {
    use super::{
        AdapterErrorId, ExistingReservation, LedgerError, LedgerErrorId, RepositoryFitLedgerState,
        ReservationDecision, ReservationRequest, ReservationToken,
    };
    use crate::repository_fit::{digest, valid_digest};
    use getrandom::fill;
    use hmac::{Hmac, Mac};
    use serde::{Deserialize, Serialize};
    use sha2::Sha256;
    use std::collections::{BTreeMap, BTreeSet};
    use std::ffi::{CStr, CString, OsStr};
    use std::fs::{self, File, OpenOptions};
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    type HmacSha256 = Hmac<Sha256>;

    const LEDGER_SCHEMA: &str = "harness-ultragoal.repository-fit-authority-ledger.v1";
    const ENVELOPE_SCHEMA: &str = "harness-ultragoal.repository-fit-authority-ledger-envelope.v1";
    const EVENT_DOMAIN: &str = "repository-fit-authority-ledger-event-v1";
    const INITIAL_HEAD_DOMAIN: &str = "repository-fit-authority-ledger-initial-head-v1";
    const AUTHORITY_DOMAIN: &str = "repository-fit-production-authority-v1";
    const LOCK_MARKER: &[u8] = b"repository-fit-authority-lock-v1\n";
    const KEY_BYTES: usize = 32;
    const MAX_LEDGER_BYTES: u64 = 64 * 1024 * 1024;
    const MAX_EVENTS: usize = 100_000;
    const MAX_STORE_ENTRIES: usize = 4;
    const KEY_NAME: &str = "authority.key";
    const LOCK_NAME: &str = "authority.lock";
    const STATE_NAME: &str = "authority-ledger.json";

    pub(super) struct FileLedger {
        store: Store,
        key_identity: FileIdentity,
        lock_identity: FileIdentity,
        store_id: String,
        authority_id: String,
        local: Mutex<ObservedHead>,
    }

    #[derive(Clone, Debug)]
    struct ObservedHead {
        generation: u64,
        head_sha256: String,
    }

    #[derive(Clone)]
    struct Store {
        requested_root: PathBuf,
        canonical_root: PathBuf,
        directory: Arc<File>,
        root_identity: RootIdentity,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct RootIdentity {
        device: u64,
        inode: u64,
        uid: u32,
        gid: u32,
        mode: u32,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct FileIdentity {
        device: u64,
        inode: u64,
        links: u64,
        uid: u32,
        gid: u32,
        mode: u32,
        length: u64,
        changed_seconds: i64,
        changed_nanoseconds: i64,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct SnapshotEnvelope {
        schema_version: String,
        payload: SnapshotPayload,
        hmac_sha256: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct SnapshotPayload {
        schema_version: String,
        store_id: String,
        authority_id: String,
        key_id: String,
        root_identity: RootIdentity,
        lock_identity: FileIdentity,
        generation: u64,
        head_sha256: String,
        events: Vec<LedgerEvent>,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct LedgerEvent {
        sequence: u64,
        event_id: String,
        prior_head_sha256: String,
        reservation_id: String,
        binding_sha256: String,
        semantic_effect_id: String,
        target_scope_id: String,
        permit_id: String,
        nonce_sha256: String,
        issued_tick: u64,
        expires_tick: u64,
        state: RepositoryFitLedgerState,
        terminal_sha256: Option<String>,
        error_id: Option<AdapterErrorId>,
        transition_tick: u64,
        event_sha256: String,
    }

    #[derive(Default)]
    struct ReplayState {
        records: BTreeMap<String, LedgerEvent>,
        nonce_owner: BTreeMap<String, String>,
        semantic_owner: BTreeMap<String, String>,
        active_targets: BTreeMap<String, String>,
    }

    struct LedgerKey([u8; KEY_BYTES]);

    struct ProcessLock(File);

    impl Drop for ProcessLock {
        fn drop(&mut self) {
            let _ = unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
        }
    }

    impl FileLedger {
        pub(super) fn open_or_initialize(root: &Path, store_id: &str) -> Result<Self, LedgerError> {
            if !valid_digest(store_id) {
                return Err(invalid_store());
            }
            let store = Store::open(root)?;
            let initial_names = store.names()?;
            let lock = if initial_names.is_empty() {
                store.open_or_create_lock()?
            } else if initial_names.contains(LOCK_NAME) {
                store.open_existing(LOCK_NAME, libc::O_RDWR)?
            } else {
                return Err(tampered());
            };
            let guard = ProcessLock::acquire(lock)?;
            store.verify_root()?;
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
            let lock_identity = exact_identity(&store, LOCK_NAME, &guard.0, 0o600)?;
            let key_file = store.open_existing(KEY_NAME, libc::O_RDONLY)?;
            let key_identity = exact_identity(&store, KEY_NAME, &key_file, 0o600)?;
            let key = read_key(&key_file)?;
            Self::finish_open(store, store_id, key, key_identity, lock_identity)
        }

        fn finish_open(
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

        pub(super) fn authority_id(&self) -> &str {
            &self.authority_id
        }

        pub(super) fn reserve(
            &self,
            request: ReservationRequest<'_>,
        ) -> Result<ReservationDecision, LedgerError> {
            validate_reservation(&request)?;
            self.with_snapshot(|payload, replayed| {
                if let Some(owner) = replayed.nonce_owner.get(request.nonce_sha256) {
                    let current = replayed.records.get(owner).ok_or_else(tampered)?;
                    if current.semantic_effect_id != request.semantic_effect_id {
                        return Err(replay_error());
                    }
                    return Ok((ReservationDecision::Existing(existing(current)), false));
                }
                if let Some(owner) = replayed.semantic_owner.get(request.semantic_effect_id) {
                    let current = replayed.records.get(owner).ok_or_else(tampered)?;
                    return Ok((ReservationDecision::Existing(existing(current)), false));
                }
                if replayed
                    .active_targets
                    .contains_key(request.target_scope_id)
                {
                    return Err(active_lease());
                }
                let reservation_id = digest(
                    &serde_json::to_vec(&(
                        "repository-fit-ledger-reservation-v1",
                        &self.authority_id,
                        request.binding_sha256,
                        request.semantic_effect_id,
                        request.target_scope_id,
                        request.permit_id,
                        request.nonce_sha256,
                    ))
                    .map_err(|_| invalid_transition())?,
                );
                let event = next_event(
                    payload,
                    &reservation_id,
                    request.binding_sha256,
                    request.semantic_effect_id,
                    request.target_scope_id,
                    request.permit_id,
                    request.nonce_sha256,
                    request.issued_tick,
                    request.expires_tick,
                    RepositoryFitLedgerState::Reserved,
                    None,
                    None,
                    request.issued_tick,
                )?;
                append(payload, event)?;
                Ok((
                    ReservationDecision::Acquired(ReservationToken {
                        reservation_id,
                        binding_sha256: request.binding_sha256.to_owned(),
                        semantic_effect_id: request.semantic_effect_id.to_owned(),
                        target_scope_id: request.target_scope_id.to_owned(),
                        permit_id: request.permit_id.to_owned(),
                        nonce_sha256: request.nonce_sha256.to_owned(),
                    }),
                    true,
                ))
            })
        }

        pub(super) fn terminal(
            &self,
            token: ReservationToken,
            state: RepositoryFitLedgerState,
            terminal_sha256: &str,
            error_id: Option<AdapterErrorId>,
            tick: u64,
        ) -> Result<(), LedgerError> {
            if !state.terminal() || !valid_digest(terminal_sha256) {
                return Err(invalid_transition());
            }
            self.with_snapshot(|payload, replayed| {
                let current = replayed
                    .records
                    .get(&token.reservation_id)
                    .ok_or_else(invalid_transition)?;
                if current.state != RepositoryFitLedgerState::Reserved
                    || current.binding_sha256 != token.binding_sha256
                    || current.semantic_effect_id != token.semantic_effect_id
                    || current.target_scope_id != token.target_scope_id
                    || current.permit_id != token.permit_id
                    || current.nonce_sha256 != token.nonce_sha256
                    || tick < current.issued_tick
                {
                    return Err(invalid_transition());
                }
                let event = next_event(
                    payload,
                    &token.reservation_id,
                    &token.binding_sha256,
                    &token.semantic_effect_id,
                    &token.target_scope_id,
                    &token.permit_id,
                    &token.nonce_sha256,
                    current.issued_tick,
                    current.expires_tick,
                    state,
                    Some(terminal_sha256),
                    error_id,
                    tick,
                )?;
                append(payload, event)?;
                Ok(((), true))
            })
        }

        fn with_snapshot<T>(
            &self,
            operation: impl FnOnce(&mut SnapshotPayload, &ReplayState) -> Result<(T, bool), LedgerError>,
        ) -> Result<T, LedgerError> {
            let mut local = self.local.lock().map_err(|_| ledger_io())?;
            self.verify_store()?;
            let lock = self.store.open_existing(LOCK_NAME, libc::O_RDWR)?;
            if exact_identity(&self.store, LOCK_NAME, &lock, 0o600)? != self.lock_identity {
                return Err(tampered());
            }
            let _guard = ProcessLock::acquire(lock)?;
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
                || (payload.generation == local.generation
                    && payload.head_sha256 != local.head_sha256)
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

        fn verify_store(&self) -> Result<(), LedgerError> {
            self.store.verify_root()?;
            self.store
                .validate_complete(self.key_identity, self.lock_identity)
        }

        #[cfg(test)]
        pub(super) fn snapshot_for_test(&self) -> Result<Vec<u8>, LedgerError> {
            self.verify_store()?;
            self.store.read_state()
        }
    }

    impl Store {
        fn open(root: &Path) -> Result<Self, LedgerError> {
            let supplied = root.to_path_buf();
            let path_metadata = fs::symlink_metadata(&supplied).map_err(|_| invalid_store())?;
            if path_metadata.file_type().is_symlink() || !path_metadata.is_dir() {
                return Err(invalid_store());
            }
            let mut options = OpenOptions::new();
            options.read(true).custom_flags(
                libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            );
            let directory = options.open(&supplied).map_err(|_| invalid_store())?;
            let metadata = directory.metadata().map_err(|_| invalid_store())?;
            let expected_uid = unsafe { libc::geteuid() };
            if !metadata.is_dir()
                || metadata.dev() != path_metadata.dev()
                || metadata.ino() != path_metadata.ino()
                || metadata.uid() != expected_uid
                || path_metadata.uid() != expected_uid
                || metadata.mode() & 0o7777 != 0o700
                || path_metadata.mode() & 0o7777 != 0o700
            {
                return Err(invalid_store());
            }
            let canonical_root = fs::canonicalize(&supplied).map_err(|_| invalid_store())?;
            if !canonical_root.is_absolute() {
                return Err(invalid_store());
            }
            let value = Self {
                requested_root: supplied,
                canonical_root,
                directory: Arc::new(directory),
                root_identity: root_identity(&metadata),
            };
            value.verify_root()?;
            Ok(value)
        }

        fn verify_root(&self) -> Result<(), LedgerError> {
            let path = fs::symlink_metadata(&self.requested_root).map_err(|_| tampered())?;
            let opened = self.directory.metadata().map_err(|_| tampered())?;
            if path.file_type().is_symlink()
                || !path.is_dir()
                || root_identity(&path) != self.root_identity
                || root_identity(&opened) != self.root_identity
                || path.uid() != unsafe { libc::geteuid() }
                || path.mode() & 0o7777 != 0o700
                || fs::canonicalize(&self.requested_root).map_err(|_| tampered())?
                    != self.canonical_root
                || descriptor_path(&self.directory)? != self.canonical_root
            {
                return Err(tampered());
            }
            Ok(())
        }

        fn names(&self) -> Result<BTreeSet<String>, LedgerError> {
            self.verify_root()?;
            let descriptor = unsafe {
                libc::openat(
                    self.directory.as_raw_fd(),
                    c".".as_ptr(),
                    libc::O_RDONLY
                        | libc::O_DIRECTORY
                        | libc::O_CLOEXEC
                        | libc::O_NOFOLLOW
                        | libc::O_NONBLOCK,
                )
            };
            if descriptor < 0 {
                return Err(ledger_io());
            }
            let stream = unsafe { libc::fdopendir(descriptor) };
            if stream.is_null() {
                unsafe { libc::close(descriptor) };
                return Err(ledger_io());
            }
            let mut names = BTreeSet::new();
            let result = loop {
                unsafe { *libc::__error() = 0 };
                let entry = unsafe { libc::readdir(stream) };
                if entry.is_null() {
                    break if unsafe { *libc::__error() } == 0 {
                        Ok(names)
                    } else {
                        Err(ledger_io())
                    };
                }
                let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
                if matches!(bytes, b"." | b"..") {
                    continue;
                }
                if names.len() >= MAX_STORE_ENTRIES {
                    break Err(tampered());
                }
                let name = std::str::from_utf8(bytes).map_err(|_| tampered())?;
                if !matches!(name, KEY_NAME | LOCK_NAME | STATE_NAME) {
                    break Err(tampered());
                }
                if !names.insert(name.to_owned()) {
                    break Err(tampered());
                }
            };
            if unsafe { libc::closedir(stream) } != 0 {
                return Err(ledger_io());
            }
            self.verify_root()?;
            result
        }

        fn open_or_create_lock(&self) -> Result<File, LedgerError> {
            match self.create_exclusive(LOCK_NAME, 0o600) {
                Ok(file) => {
                    file.sync_all().map_err(|_| ledger_io())?;
                    self.directory.sync_all().map_err(|_| ledger_io())?;
                    Ok(file)
                }
                Err(error) if error.id() == LedgerErrorId::Replay => {
                    self.open_existing(LOCK_NAME, libc::O_RDWR)
                }
                Err(error) => Err(error),
            }
        }

        fn create_key(&self) -> Result<File, LedgerError> {
            let mut bytes = [0u8; KEY_BYTES];
            fill(&mut bytes).map_err(|_| ledger_io())?;
            let mut file = self.create_exclusive(KEY_NAME, 0o600)?;
            file.write_all(&bytes).map_err(|_| ledger_io())?;
            file.sync_all().map_err(|_| ledger_io())?;
            self.directory.sync_all().map_err(|_| ledger_io())?;
            Ok(file)
        }

        fn create_exclusive(&self, name: &str, mode: u32) -> Result<File, LedgerError> {
            validate_name(name)?;
            let name = CString::new(name).map_err(|_| invalid_store())?;
            let descriptor = unsafe {
                libc::openat(
                    self.directory.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDWR
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_CLOEXEC
                        | libc::O_NOFOLLOW,
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

        fn open_existing(&self, name: &str, flags: i32) -> Result<File, LedgerError> {
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

        fn read_state(&self) -> Result<Vec<u8>, LedgerError> {
            let file = self.open_existing(STATE_NAME, libc::O_RDONLY)?;
            let _ = exact_identity(self, STATE_NAME, &file, 0o600)?;
            read_bounded(&file, MAX_LEDGER_BYTES)
        }

        fn write_initial_state(&self, bytes: &[u8]) -> Result<(), LedgerError> {
            let mut file = self.create_exclusive(STATE_NAME, 0o600)?;
            file.write_all(bytes).map_err(|_| ledger_io())?;
            file.sync_all().map_err(|_| ledger_io())?;
            self.directory.sync_all().map_err(|_| ledger_io())?;
            let _ = exact_identity(self, STATE_NAME, &file, 0o600)?;
            Ok(())
        }

        fn write_atomic_state(&self, bytes: &[u8]) -> Result<(), LedgerError> {
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

        fn validate_complete(
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

        fn exact_stat(&self, name: &str) -> Result<Option<FileIdentity>, LedgerError> {
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
        fn acquire(file: File) -> Result<Self, LedgerError> {
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
                return Err(ledger_io());
            }
            Ok(Self(file))
        }
    }

    fn initial_payload(
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

    fn authority_id(store_id: &str, key_id: &str) -> Result<String, LedgerError> {
        serde_json::to_vec(&(AUTHORITY_DOMAIN, store_id, key_id))
            .map(|bytes| digest(&bytes))
            .map_err(|_| invalid_transition())
    }

    #[allow(clippy::too_many_arguments)]
    fn decode_snapshot(
        bytes: &[u8],
        key: &LedgerKey,
        store_id: &str,
        authority_id: &str,
        key_id: &str,
        root_identity: RootIdentity,
        lock_identity: FileIdentity,
    ) -> Result<SnapshotPayload, LedgerError> {
        if bytes.is_empty() || bytes.len() as u64 > MAX_LEDGER_BYTES {
            return Err(tampered());
        }
        let envelope: SnapshotEnvelope = serde_json::from_slice(bytes).map_err(|_| tampered())?;
        if envelope.schema_version != ENVELOPE_SCHEMA
            || envelope.payload.schema_version != LEDGER_SCHEMA
            || envelope.payload.store_id != store_id
            || envelope.payload.authority_id != authority_id
            || envelope.payload.key_id != key_id
            || envelope.payload.root_identity != root_identity
            || envelope.payload.lock_identity != lock_identity
            || !valid_digest(&envelope.hmac_sha256)
        {
            return Err(tampered());
        }
        let canonical = serde_json::to_vec(&envelope.payload).map_err(|_| tampered())?;
        let mut mac = HmacSha256::new_from_slice(&key.0).map_err(|_| tampered())?;
        mac.update(&canonical);
        let supplied = decode_digest(&envelope.hmac_sha256)?;
        mac.verify_slice(&supplied).map_err(|_| tampered())?;
        Ok(envelope.payload)
    }

    fn encode_snapshot(payload: &SnapshotPayload, key: &LedgerKey) -> Result<Vec<u8>, LedgerError> {
        let canonical = serde_json::to_vec(payload).map_err(|_| invalid_transition())?;
        let mut mac = HmacSha256::new_from_slice(&key.0).map_err(|_| invalid_transition())?;
        mac.update(&canonical);
        let hmac_sha256 = format!("sha256:{:x}", mac.finalize().into_bytes());
        let bytes = serde_json::to_vec(&SnapshotEnvelope {
            schema_version: ENVELOPE_SCHEMA.to_owned(),
            payload: payload.clone(),
            hmac_sha256,
        })
        .map_err(|_| invalid_transition())?;
        if bytes.is_empty() || bytes.len() as u64 > MAX_LEDGER_BYTES {
            return Err(invalid_transition());
        }
        Ok(bytes)
    }

    fn replay(payload: &SnapshotPayload) -> Result<ReplayState, LedgerError> {
        if payload.events.len() > MAX_EVENTS
            || payload.generation != payload.events.len() as u64
            || !valid_digest(&payload.head_sha256)
        {
            return Err(tampered());
        }
        let mut state = ReplayState::default();
        let mut head = digest(
            &serde_json::to_vec(&(
                INITIAL_HEAD_DOMAIN,
                &payload.store_id,
                &payload.authority_id,
                &payload.key_id,
                payload.root_identity,
                payload.lock_identity,
            ))
            .map_err(|_| tampered())?,
        );
        for (index, event) in payload.events.iter().enumerate() {
            if event.sequence != index as u64 + 1
                || event.prior_head_sha256 != head
                || event.event_sha256 != event_digest(event)?
                || !valid_event(event)
            {
                return Err(tampered());
            }
            match event.state {
                RepositoryFitLedgerState::Reserved => {
                    if state.records.contains_key(&event.reservation_id)
                        || state.nonce_owner.contains_key(&event.nonce_sha256)
                        || state.semantic_owner.contains_key(&event.semantic_effect_id)
                        || state.active_targets.contains_key(&event.target_scope_id)
                    {
                        return Err(tampered());
                    }
                    state
                        .nonce_owner
                        .insert(event.nonce_sha256.clone(), event.reservation_id.clone());
                    state.semantic_owner.insert(
                        event.semantic_effect_id.clone(),
                        event.reservation_id.clone(),
                    );
                    state
                        .active_targets
                        .insert(event.target_scope_id.clone(), event.reservation_id.clone());
                }
                terminal => {
                    if !terminal.terminal() {
                        return Err(tampered());
                    }
                    let prior = state
                        .records
                        .get(&event.reservation_id)
                        .ok_or_else(tampered)?;
                    if prior.state != RepositoryFitLedgerState::Reserved
                        || !same_reservation(prior, event)
                        || state.active_targets.get(&event.target_scope_id)
                            != Some(&event.reservation_id)
                    {
                        return Err(tampered());
                    }
                    state.active_targets.remove(&event.target_scope_id);
                }
            }
            state
                .records
                .insert(event.reservation_id.clone(), event.clone());
            head.clone_from(&event.event_sha256);
        }
        if head != payload.head_sha256 {
            return Err(tampered());
        }
        Ok(state)
    }

    fn valid_event(event: &LedgerEvent) -> bool {
        valid_digest(&event.event_id)
            && valid_digest(&event.prior_head_sha256)
            && valid_digest(&event.reservation_id)
            && valid_digest(&event.binding_sha256)
            && valid_digest(&event.semantic_effect_id)
            && valid_digest(&event.target_scope_id)
            && valid_digest(&event.permit_id)
            && valid_digest(&event.nonce_sha256)
            && event.expires_tick >= event.issued_tick
            && match event.state {
                RepositoryFitLedgerState::Reserved => {
                    event.terminal_sha256.is_none() && event.error_id.is_none()
                }
                RepositoryFitLedgerState::Committed => {
                    event.terminal_sha256.as_deref().is_some_and(valid_digest)
                        && event.error_id.is_none()
                }
                _ => event.terminal_sha256.as_deref().is_some_and(valid_digest),
            }
    }

    fn same_reservation(left: &LedgerEvent, right: &LedgerEvent) -> bool {
        left.reservation_id == right.reservation_id
            && left.binding_sha256 == right.binding_sha256
            && left.semantic_effect_id == right.semantic_effect_id
            && left.target_scope_id == right.target_scope_id
            && left.permit_id == right.permit_id
            && left.nonce_sha256 == right.nonce_sha256
            && left.issued_tick == right.issued_tick
            && left.expires_tick == right.expires_tick
    }

    #[allow(clippy::too_many_arguments)]
    fn next_event(
        payload: &SnapshotPayload,
        reservation_id: &str,
        binding_sha256: &str,
        semantic_effect_id: &str,
        target_scope_id: &str,
        permit_id: &str,
        nonce_sha256: &str,
        issued_tick: u64,
        expires_tick: u64,
        state: RepositoryFitLedgerState,
        terminal_sha256: Option<&str>,
        error_id: Option<AdapterErrorId>,
        transition_tick: u64,
    ) -> Result<LedgerEvent, LedgerError> {
        let sequence = payload.generation.checked_add(1).ok_or_else(tampered)?;
        let event_id = digest(
            &serde_json::to_vec(&(
                "repository-fit-ledger-event-id-v1",
                &payload.authority_id,
                sequence,
                reservation_id,
                state,
            ))
            .map_err(|_| invalid_transition())?,
        );
        let mut event = LedgerEvent {
            sequence,
            event_id,
            prior_head_sha256: payload.head_sha256.clone(),
            reservation_id: reservation_id.to_owned(),
            binding_sha256: binding_sha256.to_owned(),
            semantic_effect_id: semantic_effect_id.to_owned(),
            target_scope_id: target_scope_id.to_owned(),
            permit_id: permit_id.to_owned(),
            nonce_sha256: nonce_sha256.to_owned(),
            issued_tick,
            expires_tick,
            state,
            terminal_sha256: terminal_sha256.map(str::to_owned),
            error_id,
            transition_tick,
            event_sha256: String::new(),
        };
        event.event_sha256 = event_digest(&event)?;
        if !valid_event(&event) {
            return Err(invalid_transition());
        }
        Ok(event)
    }

    fn event_digest(event: &LedgerEvent) -> Result<String, LedgerError> {
        serde_json::to_vec(&(
            EVENT_DOMAIN,
            event.sequence,
            &event.event_id,
            &event.prior_head_sha256,
            &event.reservation_id,
            &event.binding_sha256,
            &event.semantic_effect_id,
            &event.target_scope_id,
            &event.permit_id,
            &event.nonce_sha256,
            event.issued_tick,
            event.expires_tick,
            event.state,
            &event.terminal_sha256,
            event.error_id,
            event.transition_tick,
        ))
        .map(|bytes| digest(&bytes))
        .map_err(|_| invalid_transition())
    }

    fn append(payload: &mut SnapshotPayload, event: LedgerEvent) -> Result<(), LedgerError> {
        if payload.events.len() >= MAX_EVENTS || event.sequence != payload.generation + 1 {
            return Err(invalid_transition());
        }
        payload.generation = event.sequence;
        payload.head_sha256.clone_from(&event.event_sha256);
        payload.events.push(event);
        Ok(())
    }

    fn existing(event: &LedgerEvent) -> ExistingReservation {
        ExistingReservation {
            token: ReservationToken {
                reservation_id: event.reservation_id.clone(),
                binding_sha256: event.binding_sha256.clone(),
                semantic_effect_id: event.semantic_effect_id.clone(),
                target_scope_id: event.target_scope_id.clone(),
                permit_id: event.permit_id.clone(),
                nonce_sha256: event.nonce_sha256.clone(),
            },
            state: event.state,
            terminal_sha256: event.terminal_sha256.clone(),
        }
    }

    fn validate_reservation(request: &ReservationRequest<'_>) -> Result<(), LedgerError> {
        if [
            request.binding_sha256,
            request.semantic_effect_id,
            request.target_scope_id,
            request.permit_id,
            request.nonce_sha256,
        ]
        .iter()
        .any(|value| !valid_digest(value))
            || request.expires_tick < request.issued_tick
        {
            return Err(invalid_transition());
        }
        Ok(())
    }

    fn exact_identity(
        store: &Store,
        name: &str,
        file: &File,
        expected_mode: u32,
    ) -> Result<FileIdentity, LedgerError> {
        let named = store.exact_stat(name)?.ok_or_else(tampered)?;
        let opened = file_identity(&file.metadata().map_err(|_| tampered())?);
        if named != opened
            || opened.device != store.root_identity.device
            || opened.links != 1
            || opened.uid != unsafe { libc::geteuid() }
            || opened.mode & libc::S_IFMT as u32 != libc::S_IFREG as u32
            || opened.mode & 0o7777 != expected_mode
        {
            return Err(tampered());
        }
        Ok(opened)
    }

    fn read_key(file: &File) -> Result<LedgerKey, LedgerError> {
        let bytes = read_bounded(file, KEY_BYTES as u64)?;
        let bytes: [u8; KEY_BYTES] = bytes.try_into().map_err(|_| tampered())?;
        Ok(LedgerKey(bytes))
    }

    fn read_bounded(file: &File, maximum: u64) -> Result<Vec<u8>, LedgerError> {
        let metadata = file.metadata().map_err(|_| ledger_io())?;
        if !metadata.is_file() || metadata.len() > maximum {
            return Err(tampered());
        }
        let mut file = file.try_clone().map_err(|_| ledger_io())?;
        file.seek(SeekFrom::Start(0)).map_err(|_| ledger_io())?;
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        (&mut file)
            .take(maximum.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|_| ledger_io())?;
        let after = file.metadata().map_err(|_| ledger_io())?;
        if bytes.len() as u64 != metadata.len()
            || after.len() != metadata.len()
            || after.mtime() != metadata.mtime()
            || after.mtime_nsec() != metadata.mtime_nsec()
            || after.ctime() != metadata.ctime()
            || after.ctime_nsec() != metadata.ctime_nsec()
        {
            return Err(tampered());
        }
        Ok(bytes)
    }

    fn descriptor_path(file: &File) -> Result<PathBuf, LedgerError> {
        let request = format!("/dev/fd/{}", file.as_raw_fd());
        let request = CString::new(request).map_err(|_| invalid_store())?;
        let mut buffer = vec![0_i8; libc::PATH_MAX as usize];
        if unsafe {
            libc::fcntl(
                file.as_raw_fd(),
                libc::F_GETPATH,
                buffer.as_mut_ptr().cast::<libc::c_void>(),
            )
        } != 0
        {
            return Err(tampered());
        }
        let path = unsafe { CStr::from_ptr(buffer.as_ptr()) };
        if path.to_bytes().is_empty() || path.to_bytes()[0] != b'/' {
            return Err(tampered());
        }
        let _ = request;
        Ok(PathBuf::from(OsStr::from_bytes(path.to_bytes())))
    }

    fn root_identity(metadata: &fs::Metadata) -> RootIdentity {
        RootIdentity {
            device: metadata.dev(),
            inode: metadata.ino(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            mode: metadata.mode() & 0o7777,
        }
    }

    fn file_identity(metadata: &fs::Metadata) -> FileIdentity {
        FileIdentity {
            device: metadata.dev(),
            inode: metadata.ino(),
            links: metadata.nlink(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            mode: metadata.mode(),
            length: metadata.len(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        }
    }

    fn stat_identity(stat: &libc::stat) -> FileIdentity {
        FileIdentity {
            device: stat.st_dev as u64,
            inode: stat.st_ino,
            links: stat.st_nlink as u64,
            uid: stat.st_uid,
            gid: stat.st_gid,
            mode: stat.st_mode as u32,
            length: stat.st_size as u64,
            changed_seconds: stat.st_ctime,
            changed_nanoseconds: stat.st_ctime_nsec,
        }
    }

    fn temporary_name() -> Result<String, LedgerError> {
        let mut random = [0u8; 16];
        fill(&mut random).map_err(|_| ledger_io())?;
        Ok(format!(
            ".repository-fit-ledger-{}.tmp",
            random
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ))
    }

    fn rename_relative(directory: &File, source: &str, target: &str) -> Result<(), LedgerError> {
        let source = CString::new(source).map_err(|_| invalid_store())?;
        let target = CString::new(target).map_err(|_| invalid_store())?;
        if unsafe {
            libc::renameat(
                directory.as_raw_fd(),
                source.as_ptr(),
                directory.as_raw_fd(),
                target.as_ptr(),
            )
        } != 0
        {
            return Err(ledger_io());
        }
        Ok(())
    }

    fn unlink_relative(directory: &File, name: &str) -> Result<(), LedgerError> {
        let name = CString::new(name).map_err(|_| invalid_store())?;
        if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
            return Err(ledger_io());
        }
        Ok(())
    }

    fn decode_digest(value: &str) -> Result<[u8; 32], LedgerError> {
        if !valid_digest(value) {
            return Err(tampered());
        }
        let mut bytes = [0u8; 32];
        for (index, pair) in value[7..].as_bytes().chunks_exact(2).enumerate() {
            let pair = std::str::from_utf8(pair).map_err(|_| tampered())?;
            bytes[index] = u8::from_str_radix(pair, 16).map_err(|_| tampered())?;
        }
        Ok(bytes)
    }

    fn validate_name(name: &str) -> Result<(), LedgerError> {
        if name.is_empty()
            || name.len() > 255
            || name.contains('/')
            || name == "."
            || name == ".."
            || name.as_bytes().contains(&0)
        {
            return Err(invalid_store());
        }
        Ok(())
    }

    const fn invalid_store() -> LedgerError {
        LedgerError::new(LedgerErrorId::InvalidStore)
    }

    const fn tampered() -> LedgerError {
        LedgerError::new(LedgerErrorId::Tampered)
    }

    const fn replay_error() -> LedgerError {
        LedgerError::new(LedgerErrorId::Replay)
    }

    const fn active_lease() -> LedgerError {
        LedgerError::new(LedgerErrorId::ActiveLease)
    }

    const fn invalid_transition() -> LedgerError {
        LedgerError::new(LedgerErrorId::InvalidTransition)
    }

    const fn ledger_io() -> LedgerError {
        LedgerError::new(LedgerErrorId::Io)
    }
}
