use super::{
    DurableHostEffectLedger, HostEffectLedgerError, HostEffectLedgerErrorId, HostEffectLedgerHead,
    HostEffectLedgerRecord, HostEffectReservation, HostEffectState, HostEffectTransition,
    allowed_transition, is_digest,
};
use getrandom::fill;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

type HmacSha256 = Hmac<Sha256>;

const LEDGER_SCHEMA: &str = "harness-ultragoal.host-effect-ledger.v2";
const EVENT_SCHEMA: &str = "harness-ultragoal.host-effect-ledger-event.v2";
const INITIAL_HEAD_SCHEMA: &str = "harness-ultragoal.host-effect-ledger-initial-head.v2";
const KEY_BYTES: usize = 32;
const MAX_LEDGER_BYTES: u64 = 64 * 1024 * 1024;
const MAX_EVENTS: usize = 100_000;
const KEY_NAME: &str = "ledger.key";
const LOCK_NAME: &str = "ledger.lock";
const STATE_NAME: &str = "ledger.json";

/// Durable, cross-process, fail-closed implementation of the host-effect
/// ledger interface. The directory and key object are descriptor-bound at
/// construction. Every mutation publishes one authenticated, hash-chained
/// snapshot through an atomic rename and fsyncs the containing directory.
pub(crate) struct FileHostEffectLedger {
    root: PathBuf,
    canonical_root: PathBuf,
    directory: Arc<File>,
    directory_identity: FileIdentity,
    key_identity: FileIdentity,
    lock_identity: FileIdentity,
    ledger_id: String,
    local: Mutex<ObservedHead>,
    #[cfg(test)]
    lock_open_hook: Mutex<Option<LockOpenHook>>,
    #[cfg(test)]
    key_open_hook: Mutex<Option<KeyOpenHook>>,
}

#[cfg(test)]
#[derive(Clone)]
struct LockOpenHook {
    reached: Arc<std::sync::Barrier>,
    release: Arc<std::sync::Barrier>,
}

#[cfg(test)]
#[derive(Clone)]
struct KeyOpenHook {
    reached: Arc<std::sync::Barrier>,
    release: Arc<std::sync::Barrier>,
}

#[derive(Clone, Debug)]
struct ObservedHead {
    initialized: bool,
    generation: u64,
    head_sha256: String,
}

impl FileHostEffectLedger {
    pub(in crate::distribution::host_effect) fn create(
        root: &Path,
        ledger_id: String,
    ) -> Result<Self, HostEffectLedgerError> {
        validate_id(&ledger_id)?;
        create_root(root)?;
        let store = Store::open(root)?;
        let lock = store.open_or_create_lock()?;
        let lock_identity = exact_identity(&store, LOCK_NAME, &lock, 0)?;
        let guard = ProcessLock::acquire(lock)?;
        require_lock_identity(&store, &guard, lock_identity)?;
        let (key, key_identity) = store.open_or_create_key()?;
        let key_id = digest(&key.0);
        let initial = initial_payload(&ledger_id, &key_id, lock_identity)?;
        match store.exact_stat(STATE_NAME)? {
            Some(_) => {
                let bytes = store.read_exact_file(STATE_NAME, MAX_LEDGER_BYTES, 0o600)?;
                let payload = decode_snapshot(&bytes, &key, &ledger_id, &key_id)?;
                require_payload_lock(&payload, lock_identity)?;
                replay(&payload)?;
            }
            None => store.write_atomic(
                STATE_NAME,
                &encode_snapshot(&initial, &key)?,
                &guard,
                lock_identity,
            )?,
        }
        store.validate_complete(lock_identity)?;
        let bytes = store.read_exact_file(STATE_NAME, MAX_LEDGER_BYTES, 0o600)?;
        let payload = decode_snapshot(&bytes, &key, &ledger_id, &key_id)?;
        require_payload_lock(&payload, lock_identity)?;
        replay(&payload)?;
        let ledger = Self {
            root: store.root.clone(),
            canonical_root: store.canonical_root.clone(),
            directory: Arc::clone(&store.directory),
            directory_identity: store.directory_identity,
            key_identity,
            lock_identity,
            ledger_id,
            local: Mutex::new(ObservedHead {
                initialized: true,
                generation: payload.generation,
                head_sha256: payload.head_sha256,
            }),
            #[cfg(test)]
            lock_open_hook: Mutex::new(None),
            #[cfg(test)]
            key_open_hook: Mutex::new(None),
        };
        ledger.verify_store()?;
        Ok(ledger)
    }

    pub(in crate::distribution::host_effect) fn open(
        root: &Path,
        ledger_id: String,
    ) -> Result<Self, HostEffectLedgerError> {
        validate_id(&ledger_id)?;
        let store = Store::open(root)?;
        let lock = store.open_existing(LOCK_NAME, 0)?;
        let lock_identity = exact_identity(&store, LOCK_NAME, &lock, 0)?;
        let guard = ProcessLock::acquire(lock)?;
        require_lock_identity(&store, &guard, lock_identity)?;
        store.validate_complete(lock_identity)?;
        let (key, key_identity) = store.open_existing_key(|| Ok(()))?;
        let key_id = digest(&key.0);
        let bytes = store.read_exact_file(STATE_NAME, MAX_LEDGER_BYTES, 0o600)?;
        let payload = decode_snapshot(&bytes, &key, &ledger_id, &key_id)?;
        require_payload_lock(&payload, lock_identity)?;
        replay(&payload)?;
        let ledger = Self {
            root: store.root.clone(),
            canonical_root: store.canonical_root.clone(),
            directory: Arc::clone(&store.directory),
            directory_identity: store.directory_identity,
            key_identity,
            lock_identity,
            ledger_id,
            local: Mutex::new(ObservedHead {
                initialized: true,
                generation: payload.generation,
                head_sha256: payload.head_sha256,
            }),
            #[cfg(test)]
            lock_open_hook: Mutex::new(None),
            #[cfg(test)]
            key_open_hook: Mutex::new(None),
        };
        ledger.verify_store()?;
        Ok(ledger)
    }

    fn with_snapshot<T>(
        &self,
        operation: impl FnOnce(
            &mut SnapshotPayload,
            &ReplayState,
            &LedgerKey,
        ) -> Result<(T, bool), HostEffectLedgerError>,
    ) -> Result<T, HostEffectLedgerError> {
        let mut local = self.local.lock().map_err(|_| ledger_io())?;
        self.verify_store()?;
        let store = self.store();
        let lock = store.open_existing(LOCK_NAME, 0)?;
        if exact_identity(&store, LOCK_NAME, &lock, 0)? != self.lock_identity {
            return Err(tampered());
        }
        #[cfg(test)]
        self.pause_after_lock_open()?;
        let guard = ProcessLock::acquire(lock)?;
        require_lock_identity(&store, &guard, self.lock_identity)?;
        self.verify_store()?;
        store.validate_complete(self.lock_identity)?;
        let (key, identity) = store.open_existing_key(|| self.after_key_identity())?;
        if identity != self.key_identity {
            return Err(tampered());
        }
        let key_id = digest(&key.0);
        let bytes = store.read_exact_file(STATE_NAME, MAX_LEDGER_BYTES, 0o600)?;
        let mut payload = decode_snapshot(&bytes, &key, &self.ledger_id, &key_id)?;
        require_payload_lock(&payload, self.lock_identity)?;
        let replayed = replay(&payload)?;
        require_not_rolled_back(&local, &payload)?;
        let (value, changed) = operation(&mut payload, &replayed, &key)?;
        if changed {
            if payload.events.len() > MAX_EVENTS {
                return Err(invalid_record());
            }
            store.write_atomic(
                STATE_NAME,
                &encode_snapshot(&payload, &key)?,
                &guard,
                self.lock_identity,
            )?;
            let published = store.read_exact_file(STATE_NAME, MAX_LEDGER_BYTES, 0o600)?;
            let confirmed = decode_snapshot(&published, &key, &self.ledger_id, &key_id)?;
            require_payload_lock(&confirmed, self.lock_identity)?;
            replay(&confirmed)?;
            if confirmed != payload {
                return Err(tampered());
            }
        }
        store.validate_complete(self.lock_identity)?;
        local.initialized = true;
        local.generation = payload.generation;
        local.head_sha256.clone_from(&payload.head_sha256);
        self.verify_store()?;
        Ok(value)
    }

    fn store(&self) -> Store {
        Store {
            root: self.root.clone(),
            canonical_root: self.canonical_root.clone(),
            directory: Arc::clone(&self.directory),
            directory_identity: self.directory_identity,
        }
    }

    fn verify_store(&self) -> Result<(), HostEffectLedgerError> {
        let store = self.store();
        store.verify_root()?;
        if store.directory_identity != self.directory_identity {
            return Err(tampered());
        }
        if store.exact_stat(LOCK_NAME)? != Some(self.lock_identity) {
            return Err(tampered());
        }
        Ok(())
    }

    #[cfg(test)]
    fn install_lock_open_hook(&self, hook: LockOpenHook) -> Result<(), HostEffectLedgerError> {
        let mut slot = self.lock_open_hook.lock().map_err(|_| ledger_io())?;
        if slot.is_some() {
            return Err(invalid_record());
        }
        *slot = Some(hook);
        Ok(())
    }

    #[cfg(test)]
    fn install_key_open_hook(&self, hook: KeyOpenHook) -> Result<(), HostEffectLedgerError> {
        let mut slot = self.key_open_hook.lock().map_err(|_| ledger_io())?;
        if slot.is_some() {
            return Err(invalid_record());
        }
        *slot = Some(hook);
        Ok(())
    }

    #[cfg(test)]
    fn pause_after_lock_open(&self) -> Result<(), HostEffectLedgerError> {
        let hook = self.lock_open_hook.lock().map_err(|_| ledger_io())?.take();
        if let Some(hook) = hook {
            hook.reached.wait();
            hook.release.wait();
        }
        process_test_barrier()
    }

    fn after_key_identity(&self) -> Result<(), HostEffectLedgerError> {
        #[cfg(test)]
        {
            let hook = self.key_open_hook.lock().map_err(|_| ledger_io())?.take();
            if let Some(hook) = hook {
                hook.reached.wait();
                hook.release.wait();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
fn process_test_barrier() -> Result<(), HostEffectLedgerError> {
    let Some(root) = std::env::var_os("HUL_LEDGER_CHILD_BARRIER") else {
        return Ok(());
    };
    let root = PathBuf::from(root);
    if !root.is_absolute() {
        return Err(invalid_record());
    }
    let root_identity =
        FileIdentity::from_metadata(&fs::symlink_metadata(&root).map_err(|_| ledger_io())?);
    validate_directory(root_identity)?;
    let value = std::env::var("HUL_LEDGER_CHILD_VALUE").map_err(|_| invalid_record())?;
    if value.len() != 1 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid_record());
    }
    let ready = root.join(format!("ready-{value}"));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&ready)
        .map_err(|_| ledger_io())?;
    file.sync_all().map_err(|_| ledger_io())?;
    let ready_identity = FileIdentity::from_metadata(&file.metadata().map_err(|_| ledger_io())?);
    validate_regular(ready_identity, 0, 0o600)?;
    let release = root.join("release");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        match fs::symlink_metadata(&release) {
            Ok(metadata) => {
                let identity = FileIdentity::from_metadata(&metadata);
                validate_regular(identity, 0, 0o600)?;
                if identity.length != 0 {
                    return Err(tampered());
                }
                return Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if std::time::Instant::now() >= deadline {
                    return Err(ledger_io());
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            Err(_) => return Err(ledger_io()),
        }
    }
}

impl DurableHostEffectLedger for FileHostEffectLedger {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError> {
        self.with_snapshot(|payload, _, _| {
            Ok((
                HostEffectLedgerHead::new(payload.generation, payload.head_sha256.clone())?,
                false,
            ))
        })
    }

    fn reserve(
        &self,
        reservation: HostEffectReservation,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        self.with_snapshot(|payload, replayed, _| {
            validate_reservation(&reservation, &self.ledger_id)?;
            if reservation.expected_head_sha256 != payload.head_sha256 {
                return Err(stale_head());
            }
            if replayed.records.contains_key(&reservation.permit_id)
                || replayed.nonces.contains(&reservation.nonce_sha256)
                || replayed
                    .semantic_keys
                    .contains(&reservation.semantic_key_sha256)
            {
                return Err(replay_error());
            }
            let event = reserve_event(payload, &reservation)?;
            payload.events.push(event);
            payload.generation += 1;
            payload.head_sha256 = payload
                .events
                .last()
                .ok_or_else(invalid_record)?
                .current_head_sha256
                .clone();
            let next = replay(payload)?;
            let record = next
                .records
                .get(&reservation.permit_id)
                .ok_or_else(invalid_record)?
                .to_runtime()?;
            Ok((record, true))
        })
    }

    fn transition(
        &self,
        transition: HostEffectTransition,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        self.with_snapshot(|payload, replayed, _| {
            if transition.expected_head.generation != payload.generation
                || transition.expected_head.head_sha256 != payload.head_sha256
            {
                return Err(stale_head());
            }
            let current = replayed.records.get(&transition.permit_id).ok_or_else(|| {
                HostEffectLedgerError::new(HostEffectLedgerErrorId::InvalidRecord)
            })?;
            if current.state != transition.expected_state
                || !allowed_transition(transition.expected_state, transition.next_state)
            {
                return Err(invalid_transition());
            }
            let requires_outcome = matches!(
                transition.next_state,
                HostEffectState::Settled | HostEffectState::Failed | HostEffectState::Ambiguous
            );
            if transition.outcome_sha256.is_some() != requires_outcome
                || transition
                    .outcome_sha256
                    .as_ref()
                    .is_some_and(|value| !is_digest(value))
            {
                return Err(invalid_transition());
            }
            let event = transition_event(payload, &transition)?;
            payload.events.push(event);
            payload.generation += 1;
            payload.head_sha256 = payload
                .events
                .last()
                .ok_or_else(invalid_record)?
                .current_head_sha256
                .clone();
            let next = replay(payload)?;
            let record = next
                .records
                .get(&transition.permit_id)
                .ok_or_else(invalid_record)?
                .to_runtime()?;
            Ok((record, true))
        })
    }

    fn read(
        &self,
        permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError> {
        if !is_digest(permit_id) {
            return Err(invalid_record());
        }
        self.with_snapshot(|_, replayed, _| {
            Ok((
                replayed
                    .records
                    .get(permit_id)
                    .map(CurrentRecord::to_runtime)
                    .transpose()?,
                false,
            ))
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedSnapshot {
    payload: SnapshotPayload,
    mac_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotPayload {
    schema: String,
    ledger_id: String,
    ledger_key_id: String,
    lock_identity: FileIdentity,
    generation: u64,
    head_sha256: String,
    events: Vec<PersistedEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedEvent {
    schema: String,
    generation: u64,
    prior_head_sha256: String,
    current_head_sha256: String,
    permit_id: String,
    reservation: Option<PersistedReservation>,
    expected_state: Option<String>,
    next_state: String,
    outcome_sha256: Option<String>,
    record_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedReservation {
    issuer_id: String,
    ledger_id: String,
    key_id: String,
    permit_id: String,
    semantic_key_sha256: String,
    nonce_sha256: String,
    binding_sha256: String,
    expected_head_sha256: String,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
}

#[derive(Default)]
struct ReplayState {
    records: BTreeMap<String, CurrentRecord>,
    nonces: BTreeSet<String>,
    semantic_keys: BTreeSet<String>,
}

#[derive(Clone)]
struct CurrentRecord {
    reservation: PersistedReservation,
    state: HostEffectState,
    record_sha256: String,
    prior_head: HostEffectLedgerHead,
    current_head: HostEffectLedgerHead,
    outcome_sha256: Option<String>,
}

impl CurrentRecord {
    fn to_runtime(&self) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        Ok(HostEffectLedgerRecord {
            reservation: self.reservation.to_runtime()?,
            state: self.state,
            record_sha256: self.record_sha256.clone(),
            prior_head: self.prior_head.clone(),
            current_head: self.current_head.clone(),
            outcome_sha256: self.outcome_sha256.clone(),
        })
    }
}

impl PersistedReservation {
    fn from_runtime(value: &HostEffectReservation) -> Self {
        Self {
            issuer_id: value.issuer_id.clone(),
            ledger_id: value.ledger_id.clone(),
            key_id: value.key_id.clone(),
            permit_id: value.permit_id.clone(),
            semantic_key_sha256: value.semantic_key_sha256.clone(),
            nonce_sha256: value.nonce_sha256.clone(),
            binding_sha256: value.binding_sha256.clone(),
            expected_head_sha256: value.expected_head_sha256.clone(),
            issued_at_unix_ms: value.issued_at_unix_ms,
            expires_at_unix_ms: value.expires_at_unix_ms,
        }
    }

    fn to_runtime(&self) -> Result<HostEffectReservation, HostEffectLedgerError> {
        validate_persisted_reservation(self)?;
        Ok(HostEffectReservation {
            issuer_id: self.issuer_id.clone(),
            ledger_id: self.ledger_id.clone(),
            key_id: self.key_id.clone(),
            permit_id: self.permit_id.clone(),
            semantic_key_sha256: self.semantic_key_sha256.clone(),
            nonce_sha256: self.nonce_sha256.clone(),
            binding_sha256: self.binding_sha256.clone(),
            expected_head_sha256: self.expected_head_sha256.clone(),
            issued_at_unix_ms: self.issued_at_unix_ms,
            expires_at_unix_ms: self.expires_at_unix_ms,
        })
    }
}

fn initial_payload(
    ledger_id: &str,
    key_id: &str,
    lock_identity: FileIdentity,
) -> Result<SnapshotPayload, HostEffectLedgerError> {
    #[derive(Serialize)]
    struct InitialHead<'a> {
        schema: &'static str,
        ledger_id: &'a str,
        ledger_key_id: &'a str,
        lock_identity: FileIdentity,
    }
    let head_sha256 = digest_json(&InitialHead {
        schema: INITIAL_HEAD_SCHEMA,
        ledger_id,
        ledger_key_id: key_id,
        lock_identity,
    })?;
    Ok(SnapshotPayload {
        schema: LEDGER_SCHEMA.to_owned(),
        ledger_id: ledger_id.to_owned(),
        ledger_key_id: key_id.to_owned(),
        lock_identity,
        generation: 0,
        head_sha256,
        events: Vec::new(),
    })
}

fn reserve_event(
    payload: &SnapshotPayload,
    reservation: &HostEffectReservation,
) -> Result<PersistedEvent, HostEffectLedgerError> {
    event(
        payload,
        reservation.permit_id.clone(),
        Some(PersistedReservation::from_runtime(reservation)),
        None,
        HostEffectState::Reserved,
        None,
    )
}

fn transition_event(
    payload: &SnapshotPayload,
    transition: &HostEffectTransition,
) -> Result<PersistedEvent, HostEffectLedgerError> {
    event(
        payload,
        transition.permit_id.clone(),
        None,
        Some(transition.expected_state),
        transition.next_state,
        transition.outcome_sha256.clone(),
    )
}

fn event(
    payload: &SnapshotPayload,
    permit_id: String,
    reservation: Option<PersistedReservation>,
    expected_state: Option<HostEffectState>,
    next_state: HostEffectState,
    outcome_sha256: Option<String>,
) -> Result<PersistedEvent, HostEffectLedgerError> {
    let generation = payload
        .generation
        .checked_add(1)
        .ok_or_else(invalid_record)?;
    let expected_state = expected_state.map(state_name).map(str::to_owned);
    let next_state = state_name(next_state).to_owned();
    #[derive(Serialize)]
    struct RecordPreimage<'a> {
        schema: &'static str,
        ledger_id: &'a str,
        ledger_key_id: &'a str,
        lock_identity: FileIdentity,
        generation: u64,
        prior_head_sha256: &'a str,
        permit_id: &'a str,
        reservation: &'a Option<PersistedReservation>,
        expected_state: &'a Option<String>,
        next_state: &'a str,
        outcome_sha256: &'a Option<String>,
    }
    let record_sha256 = digest_json(&RecordPreimage {
        schema: EVENT_SCHEMA,
        ledger_id: &payload.ledger_id,
        ledger_key_id: &payload.ledger_key_id,
        lock_identity: payload.lock_identity,
        generation,
        prior_head_sha256: &payload.head_sha256,
        permit_id: &permit_id,
        reservation: &reservation,
        expected_state: &expected_state,
        next_state: &next_state,
        outcome_sha256: &outcome_sha256,
    })?;
    #[derive(Serialize)]
    struct HeadPreimage<'a> {
        schema: &'static str,
        ledger_id: &'a str,
        ledger_key_id: &'a str,
        lock_identity: FileIdentity,
        generation: u64,
        prior_head_sha256: &'a str,
        permit_id: &'a str,
        record_sha256: &'a str,
    }
    let current_head_sha256 = digest_json(&HeadPreimage {
        schema: "harness-ultragoal.host-effect-ledger-head.v2",
        ledger_id: &payload.ledger_id,
        ledger_key_id: &payload.ledger_key_id,
        lock_identity: payload.lock_identity,
        generation,
        prior_head_sha256: &payload.head_sha256,
        permit_id: &permit_id,
        record_sha256: &record_sha256,
    })?;
    Ok(PersistedEvent {
        schema: EVENT_SCHEMA.to_owned(),
        generation,
        prior_head_sha256: payload.head_sha256.clone(),
        current_head_sha256,
        permit_id,
        reservation,
        expected_state,
        next_state,
        outcome_sha256,
        record_sha256,
    })
}

fn replay(payload: &SnapshotPayload) -> Result<ReplayState, HostEffectLedgerError> {
    if payload.schema != LEDGER_SCHEMA
        || !valid_id(&payload.ledger_id)
        || !is_digest(&payload.ledger_key_id)
        || validate_lock_identity(payload.lock_identity).is_err()
        || !is_digest(&payload.head_sha256)
        || payload.events.len() > MAX_EVENTS
        || payload.generation != payload.events.len() as u64
    {
        return Err(tampered());
    }
    let initial = initial_payload(
        &payload.ledger_id,
        &payload.ledger_key_id,
        payload.lock_identity,
    )?;
    let mut expected_head = initial.head_sha256;
    let mut state = ReplayState::default();
    for (index, row) in payload.events.iter().enumerate() {
        let generation = index as u64 + 1;
        if row.schema != EVENT_SCHEMA
            || row.generation != generation
            || row.prior_head_sha256 != expected_head
            || !is_digest(&row.permit_id)
            || !is_digest(&row.record_sha256)
            || !is_digest(&row.current_head_sha256)
            || row
                .outcome_sha256
                .as_ref()
                .is_some_and(|value| !is_digest(value))
        {
            return Err(tampered());
        }
        let next_state = parse_state(&row.next_state)?;
        let expected_state = row.expected_state.as_deref().map(parse_state).transpose()?;
        let generated = event(
            &SnapshotPayload {
                schema: payload.schema.clone(),
                ledger_id: payload.ledger_id.clone(),
                ledger_key_id: payload.ledger_key_id.clone(),
                lock_identity: payload.lock_identity,
                generation: generation - 1,
                head_sha256: expected_head.clone(),
                events: Vec::new(),
            },
            row.permit_id.clone(),
            row.reservation.clone(),
            expected_state,
            next_state,
            row.outcome_sha256.clone(),
        )?;
        if generated != *row {
            return Err(tampered());
        }
        match (&row.reservation, expected_state) {
            (Some(reservation), None) if next_state == HostEffectState::Reserved => {
                validate_persisted_reservation(reservation)?;
                if reservation.permit_id != row.permit_id
                    || reservation.ledger_id != payload.ledger_id
                    || state.records.contains_key(&row.permit_id)
                    || !state.nonces.insert(reservation.nonce_sha256.clone())
                    || !state
                        .semantic_keys
                        .insert(reservation.semantic_key_sha256.clone())
                {
                    return Err(tampered());
                }
                state.records.insert(
                    row.permit_id.clone(),
                    CurrentRecord {
                        reservation: reservation.clone(),
                        state: HostEffectState::Reserved,
                        record_sha256: row.record_sha256.clone(),
                        prior_head: HostEffectLedgerHead::new(
                            generation - 1,
                            row.prior_head_sha256.clone(),
                        )?,
                        current_head: HostEffectLedgerHead::new(
                            generation,
                            row.current_head_sha256.clone(),
                        )?,
                        outcome_sha256: None,
                    },
                );
            }
            (None, Some(expected)) => {
                let current = state.records.get_mut(&row.permit_id).ok_or_else(tampered)?;
                let requires_outcome = matches!(
                    next_state,
                    HostEffectState::Settled | HostEffectState::Failed | HostEffectState::Ambiguous
                );
                if current.state != expected
                    || !allowed_transition(expected, next_state)
                    || row.outcome_sha256.is_some() != requires_outcome
                {
                    return Err(tampered());
                }
                current.state = next_state;
                current.record_sha256.clone_from(&row.record_sha256);
                current.prior_head =
                    HostEffectLedgerHead::new(generation - 1, row.prior_head_sha256.clone())?;
                current.current_head =
                    HostEffectLedgerHead::new(generation, row.current_head_sha256.clone())?;
                current.outcome_sha256.clone_from(&row.outcome_sha256);
            }
            _ => return Err(tampered()),
        }
        expected_head.clone_from(&row.current_head_sha256);
    }
    if payload.head_sha256 != expected_head {
        return Err(tampered());
    }
    Ok(state)
}

fn encode_snapshot(
    payload: &SnapshotPayload,
    key: &LedgerKey,
) -> Result<Vec<u8>, HostEffectLedgerError> {
    replay(payload)?;
    let payload_bytes = serde_json::to_vec(payload).map_err(|_| invalid_record())?;
    let mac_sha256 = sign(&key.0, &payload_bytes)?;
    let mut bytes = serde_json::to_vec(&SignedSnapshot {
        payload: payload.clone(),
        mac_sha256,
    })
    .map_err(|_| invalid_record())?;
    bytes.push(b'\n');
    if bytes.len() as u64 > MAX_LEDGER_BYTES {
        return Err(invalid_record());
    }
    Ok(bytes)
}

fn decode_snapshot(
    bytes: &[u8],
    key: &LedgerKey,
    ledger_id: &str,
    key_id: &str,
) -> Result<SnapshotPayload, HostEffectLedgerError> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_LEDGER_BYTES || !bytes.ends_with(b"\n") {
        return Err(tampered());
    }
    let signed: SignedSnapshot = serde_json::from_slice(bytes).map_err(|_| tampered())?;
    if signed.payload.ledger_id != ledger_id
        || signed.payload.ledger_key_id != key_id
        || !is_digest(&signed.mac_sha256)
    {
        return Err(tampered());
    }
    let payload_bytes = serde_json::to_vec(&signed.payload).map_err(|_| tampered())?;
    verify_mac(&key.0, &payload_bytes, &signed.mac_sha256)?;
    let canonical = encode_snapshot(&signed.payload, key)?;
    if canonical != bytes {
        return Err(tampered());
    }
    Ok(signed.payload)
}

fn validate_reservation(
    value: &HostEffectReservation,
    ledger_id: &str,
) -> Result<(), HostEffectLedgerError> {
    validate_persisted_reservation(&PersistedReservation::from_runtime(value))?;
    if value.ledger_id != ledger_id {
        return Err(invalid_record());
    }
    Ok(())
}

fn validate_persisted_reservation(
    value: &PersistedReservation,
) -> Result<(), HostEffectLedgerError> {
    if !valid_id(&value.issuer_id)
        || !valid_id(&value.ledger_id)
        || !is_digest(&value.key_id)
        || !is_digest(&value.permit_id)
        || !is_digest(&value.semantic_key_sha256)
        || !is_digest(&value.nonce_sha256)
        || !is_digest(&value.binding_sha256)
        || !is_digest(&value.expected_head_sha256)
        || value.issued_at_unix_ms >= value.expires_at_unix_ms
    {
        return Err(invalid_record());
    }
    Ok(())
}

fn require_not_rolled_back(
    observed: &ObservedHead,
    payload: &SnapshotPayload,
) -> Result<(), HostEffectLedgerError> {
    if observed.initialized
        && (payload.generation < observed.generation
            || (payload.generation == observed.generation
                && payload.head_sha256 != observed.head_sha256))
    {
        return Err(tampered());
    }
    Ok(())
}

fn state_name(state: HostEffectState) -> &'static str {
    match state {
        HostEffectState::Reserved => "reserved",
        HostEffectState::InFlight => "in-flight",
        HostEffectState::Settled => "settled",
        HostEffectState::Failed => "failed",
        HostEffectState::Ambiguous => "ambiguous",
    }
}

fn parse_state(value: &str) -> Result<HostEffectState, HostEffectLedgerError> {
    match value {
        "reserved" => Ok(HostEffectState::Reserved),
        "in-flight" => Ok(HostEffectState::InFlight),
        "settled" => Ok(HostEffectState::Settled),
        "failed" => Ok(HostEffectState::Failed),
        "ambiguous" => Ok(HostEffectState::Ambiguous),
        _ => Err(tampered()),
    }
}

fn sign(key: &[u8], bytes: &[u8]) -> Result<String, HostEffectLedgerError> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| invalid_record())?;
    mac.update(bytes);
    Ok(format!("sha256:{:x}", mac.finalize().into_bytes()))
}

fn verify_mac(key: &[u8], bytes: &[u8], expected: &str) -> Result<(), HostEffectLedgerError> {
    let raw = decode_digest(expected)?;
    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| tampered())?;
    mac.update(bytes);
    mac.verify_slice(&raw).map_err(|_| tampered())
}

fn decode_digest(value: &str) -> Result<[u8; 32], HostEffectLedgerError> {
    if !is_digest(value) {
        return Err(tampered());
    }
    let mut bytes = [0_u8; 32];
    for (index, pair) in value[7..].as_bytes().chunks_exact(2).enumerate() {
        let high = hex(pair[0]).ok_or_else(tampered)?;
        let low = hex(pair[1]).ok_or_else(tampered)?;
        bytes[index] = high << 4 | low;
    }
    Ok(bytes)
}

fn hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn digest_json(value: &impl Serialize) -> Result<String, HostEffectLedgerError> {
    serde_json::to_vec(value)
        .map(|bytes| digest(&bytes))
        .map_err(|_| invalid_record())
}

struct LedgerKey([u8; KEY_BYTES]);

impl Drop for LedgerKey {
    fn drop(&mut self) {
        for byte in &mut self.0 {
            unsafe { std::ptr::write_volatile(byte, 0) };
        }
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FileIdentity {
    device: u64,
    inode: u64,
    mode: u32,
    hard_links: u64,
    length: u64,
}

impl FileIdentity {
    fn from_metadata(metadata: &fs::Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            mode: metadata.mode(),
            hard_links: metadata.nlink(),
            length: metadata.len(),
        }
    }

    fn regular(self) -> bool {
        self.mode & u32::from(libc::S_IFMT) == u32::from(libc::S_IFREG)
    }

    fn directory(self) -> bool {
        self.mode & u32::from(libc::S_IFMT) == u32::from(libc::S_IFDIR)
    }

    fn permissions(self) -> u32 {
        self.mode & 0o777
    }

    fn same_directory_anchor(self, other: Self) -> bool {
        self.device == other.device && self.inode == other.inode && self.mode == other.mode
    }
}

struct Store {
    root: PathBuf,
    canonical_root: PathBuf,
    directory: Arc<File>,
    directory_identity: FileIdentity,
}

impl Store {
    fn open(root: &Path) -> Result<Self, HostEffectLedgerError> {
        let named = fs::symlink_metadata(root).map_err(|_| ledger_io())?;
        let named_identity = FileIdentity::from_metadata(&named);
        validate_directory(named_identity)?;
        let mut options = OpenOptions::new();
        options.read(true).custom_flags(
            libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        );
        let directory = options.open(root).map_err(|_| ledger_io())?;
        let descriptor = directory.metadata().map_err(|_| ledger_io())?;
        let directory_identity = FileIdentity::from_metadata(&descriptor);
        validate_directory(directory_identity)?;
        if named_identity != directory_identity {
            return Err(tampered());
        }
        let store = Self {
            root: root.to_path_buf(),
            canonical_root: fs::canonicalize(root).map_err(|_| ledger_io())?,
            directory: Arc::new(directory),
            directory_identity,
        };
        store.verify_root()?;
        Ok(store)
    }

    fn verify_root(&self) -> Result<(), HostEffectLedgerError> {
        let named = fs::symlink_metadata(&self.root).map_err(|_| tampered())?;
        let descriptor = self.directory.metadata().map_err(|_| ledger_io())?;
        let named_identity = FileIdentity::from_metadata(&named);
        let descriptor_identity = FileIdentity::from_metadata(&descriptor);
        validate_directory(named_identity)?;
        validate_directory(descriptor_identity)?;
        if !named_identity.same_directory_anchor(self.directory_identity)
            || !descriptor_identity.same_directory_anchor(self.directory_identity)
            || fs::canonicalize(&self.root).map_err(|_| tampered())? != self.canonical_root
        {
            return Err(tampered());
        }
        Ok(())
    }

    fn validate_complete(
        &self,
        expected_lock_identity: FileIdentity,
    ) -> Result<(), HostEffectLedgerError> {
        self.verify_root()?;
        let mut observed = BTreeSet::new();
        for entry in fs::read_dir(&self.root).map_err(|_| ledger_io())? {
            let entry = entry.map_err(|_| ledger_io())?;
            let name = entry.file_name().into_string().map_err(|_| tampered())?;
            if !observed.insert(name.clone()) {
                return Err(tampered());
            }
            let identity = self.exact_stat(&name)?.ok_or_else(tampered)?;
            match name.as_str() {
                LOCK_NAME => {
                    validate_lock_identity(identity)?;
                    if identity != expected_lock_identity {
                        return Err(tampered());
                    }
                }
                KEY_NAME => {
                    validate_regular(identity, KEY_BYTES as u64, 0o600)?;
                    if identity.length != KEY_BYTES as u64 {
                        return Err(tampered());
                    }
                }
                STATE_NAME => {
                    validate_regular(identity, MAX_LEDGER_BYTES, 0o600)?;
                    if identity.length == 0 {
                        return Err(tampered());
                    }
                }
                _ => return Err(tampered()),
            }
        }
        let expected = BTreeSet::from([
            KEY_NAME.to_owned(),
            LOCK_NAME.to_owned(),
            STATE_NAME.to_owned(),
        ]);
        self.verify_root()?;
        if observed != expected {
            return Err(tampered());
        }
        Ok(())
    }

    fn open_or_create_lock(&self) -> Result<File, HostEffectLedgerError> {
        self.verify_root()?;
        let file = open_relative(
            &self.directory,
            LOCK_NAME,
            libc::O_RDWR | libc::O_CREAT | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )?;
        exact_identity(self, LOCK_NAME, &file, 0)?;
        self.directory.sync_all().map_err(|_| ledger_io())?;
        Ok(file)
    }

    fn open_existing(&self, name: &str, max_bytes: u64) -> Result<File, HostEffectLedgerError> {
        self.verify_root()?;
        let access = if name == LOCK_NAME {
            libc::O_RDWR
        } else {
            libc::O_RDONLY
        };
        let file = open_relative(
            &self.directory,
            name,
            access | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0,
        )?;
        exact_identity(self, name, &file, max_bytes)?;
        Ok(file)
    }

    fn open_or_create_key(&self) -> Result<(LedgerKey, FileIdentity), HostEffectLedgerError> {
        self.verify_root()?;
        if self.exact_stat(KEY_NAME)?.is_none() {
            let mut bytes = [0_u8; KEY_BYTES];
            fill(&mut bytes).map_err(|_| ledger_io())?;
            let created = open_relative(
                &self.directory,
                KEY_NAME,
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            );
            match created {
                Ok(mut file) => {
                    file.write_all(&bytes).map_err(|_| ledger_io())?;
                    file.sync_all().map_err(|_| ledger_io())?;
                    self.directory.sync_all().map_err(|_| ledger_io())?;
                }
                Err(_error) if self.exact_stat(KEY_NAME)?.is_some() => {}
                Err(error) => return Err(error),
            }
            for byte in &mut bytes {
                unsafe { std::ptr::write_volatile(byte, 0) };
            }
        }
        self.open_existing_key(|| Ok(()))
    }

    fn open_existing_key(
        &self,
        after_identity: impl FnOnce() -> Result<(), HostEffectLedgerError>,
    ) -> Result<(LedgerKey, FileIdentity), HostEffectLedgerError> {
        let mut file = self.open_existing(KEY_NAME, KEY_BYTES as u64)?;
        let identity = exact_identity(self, KEY_NAME, &file, KEY_BYTES as u64)?;
        if identity.length != KEY_BYTES as u64 {
            return Err(tampered());
        }
        after_identity()?;
        let bytes = self.read_bound_file(KEY_NAME, &mut file, identity, KEY_BYTES as u64, 0o600)?;
        let key: [u8; KEY_BYTES] = bytes.try_into().map_err(|_| tampered())?;
        Ok((LedgerKey(key), identity))
    }

    fn read_exact_file(
        &self,
        name: &str,
        max_bytes: u64,
        required_mode: u32,
    ) -> Result<Vec<u8>, HostEffectLedgerError> {
        self.verify_root()?;
        let mut file = self.open_existing(name, max_bytes)?;
        let identity = exact_identity(self, name, &file, max_bytes)?;
        self.read_bound_file(name, &mut file, identity, max_bytes, required_mode)
    }

    fn read_bound_file(
        &self,
        name: &str,
        file: &mut File,
        expected_identity: FileIdentity,
        max_bytes: u64,
        required_mode: u32,
    ) -> Result<Vec<u8>, HostEffectLedgerError> {
        self.verify_root()?;
        let descriptor_before =
            FileIdentity::from_metadata(&file.metadata().map_err(|_| ledger_io())?);
        let named_before = self.exact_stat(name)?.ok_or_else(tampered)?;
        validate_regular(descriptor_before, max_bytes, required_mode)?;
        if descriptor_before != expected_identity || named_before != expected_identity {
            return Err(tampered());
        }
        let mut bytes = Vec::with_capacity(expected_identity.length.min(max_bytes) as usize);
        Read::by_ref(file)
            .take(max_bytes.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|_| ledger_io())?;
        let descriptor_after =
            FileIdentity::from_metadata(&file.metadata().map_err(|_| ledger_io())?);
        let named_after = self.exact_stat(name)?.ok_or_else(tampered)?;
        if bytes.len() as u64 > max_bytes
            || bytes.len() as u64 != descriptor_after.length
            || descriptor_after != expected_identity
            || descriptor_after != named_after
        {
            return Err(tampered());
        }
        self.verify_root()?;
        Ok(bytes)
    }

    fn write_atomic(
        &self,
        name: &str,
        bytes: &[u8],
        lock: &ProcessLock,
        expected_lock_identity: FileIdentity,
    ) -> Result<(), HostEffectLedgerError> {
        if bytes.is_empty() || bytes.len() as u64 > MAX_LEDGER_BYTES {
            return Err(invalid_record());
        }
        self.verify_root()?;
        let target_before = self.exact_stat(name)?;
        if target_before
            .is_some_and(|identity| validate_regular(identity, MAX_LEDGER_BYTES, 0o600).is_err())
        {
            return Err(tampered());
        }
        let (temp_name, mut temp) = self.create_temp(name)?;
        let result = (|| {
            temp.write_all(bytes).map_err(|_| ledger_io())?;
            temp.sync_all().map_err(|_| ledger_io())?;
            let temp_identity = exact_identity(self, &temp_name, &temp, bytes.len() as u64)?;
            validate_regular(temp_identity, bytes.len() as u64, 0o600)?;
            if temp_identity.length != bytes.len() as u64 {
                return Err(tampered());
            }
            self.verify_root()?;
            if self.exact_stat(name)? != target_before {
                return Err(stale_head());
            }
            require_lock_identity(self, lock, expected_lock_identity)?;
            rename_relative(&self.directory, &temp_name, name)?;
            self.directory.sync_all().map_err(|_| ledger_io())?;
            require_lock_identity(self, lock, expected_lock_identity)?;
            self.verify_root()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = unlink_relative(&self.directory, &temp_name);
        }
        result
    }

    fn create_temp(&self, target: &str) -> Result<(String, File), HostEffectLedgerError> {
        for _ in 0..64 {
            let mut nonce = [0_u8; 8];
            fill(&mut nonce).map_err(|_| ledger_io())?;
            let name = format!(".{target}.{:x}.tmp", u64::from_ne_bytes(nonce));
            match open_relative(
                &self.directory,
                &name,
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            ) {
                Ok(file) => return Ok((name, file)),
                Err(_) if self.exact_stat(&name)?.is_some() => continue,
                Err(error) => return Err(error),
            }
        }
        Err(ledger_io())
    }

    fn exact_stat(&self, name: &str) -> Result<Option<FileIdentity>, HostEffectLedgerError> {
        self.verify_root_shallow()?;
        stat_relative(&self.directory, name)
    }

    fn verify_root_shallow(&self) -> Result<(), HostEffectLedgerError> {
        let descriptor = self.directory.metadata().map_err(|_| ledger_io())?;
        if !FileIdentity::from_metadata(&descriptor).same_directory_anchor(self.directory_identity)
        {
            return Err(tampered());
        }
        Ok(())
    }
}

struct ProcessLock {
    file: File,
}

impl ProcessLock {
    fn acquire(file: File) -> Result<Self, HostEffectLedgerError> {
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return Err(ledger_io());
        }
        Ok(Self { file })
    }

    fn file(&self) -> &File {
        &self.file
    }
}

impl Drop for ProcessLock {
    fn drop(&mut self) {
        unsafe {
            libc::flock(self.file.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

fn create_root(root: &Path) -> Result<(), HostEffectLedgerError> {
    match fs::symlink_metadata(root) {
        Ok(metadata) => validate_directory(FileIdentity::from_metadata(&metadata)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::DirBuilder::new()
                .mode(0o700)
                .create(root)
                .map_err(|_| ledger_io())?;
            let metadata = fs::symlink_metadata(root).map_err(|_| ledger_io())?;
            validate_directory(FileIdentity::from_metadata(&metadata))
        }
        Err(_) => Err(ledger_io()),
    }
}

fn validate_directory(identity: FileIdentity) -> Result<(), HostEffectLedgerError> {
    if !identity.directory() || identity.hard_links == 0 || identity.permissions() != 0o700 {
        return Err(tampered());
    }
    Ok(())
}

fn validate_regular(
    identity: FileIdentity,
    max_bytes: u64,
    required_mode: u32,
) -> Result<(), HostEffectLedgerError> {
    if !identity.regular()
        || identity.hard_links != 1
        || identity.length > max_bytes
        || identity.permissions() != required_mode
    {
        return Err(tampered());
    }
    Ok(())
}

fn validate_lock_identity(identity: FileIdentity) -> Result<(), HostEffectLedgerError> {
    validate_regular(identity, 0, 0o600)?;
    if identity.length != 0 {
        return Err(tampered());
    }
    Ok(())
}

fn require_payload_lock(
    payload: &SnapshotPayload,
    expected: FileIdentity,
) -> Result<(), HostEffectLedgerError> {
    validate_lock_identity(payload.lock_identity)?;
    if payload.lock_identity != expected {
        return Err(tampered());
    }
    Ok(())
}

fn require_lock_identity(
    store: &Store,
    lock: &ProcessLock,
    expected: FileIdentity,
) -> Result<(), HostEffectLedgerError> {
    let observed = exact_identity(store, LOCK_NAME, lock.file(), 0)?;
    validate_lock_identity(observed)?;
    if observed != expected {
        return Err(tampered());
    }
    Ok(())
}

fn exact_identity(
    store: &Store,
    name: &str,
    file: &File,
    max_bytes: u64,
) -> Result<FileIdentity, HostEffectLedgerError> {
    let descriptor = FileIdentity::from_metadata(&file.metadata().map_err(|_| ledger_io())?);
    let named = store.exact_stat(name)?.ok_or_else(tampered)?;
    validate_regular(descriptor, max_bytes, 0o600)?;
    if descriptor != named {
        return Err(tampered());
    }
    Ok(descriptor)
}

fn open_relative(
    directory: &File,
    name: &str,
    flags: i32,
    mode: u32,
) -> Result<File, HostEffectLedgerError> {
    validate_name(name)?;
    let name = CString::new(name).map_err(|_| invalid_record())?;
    let descriptor = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags, mode) };
    if descriptor < 0 {
        return Err(ledger_io());
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

fn stat_relative(
    directory: &File,
    name: &str,
) -> Result<Option<FileIdentity>, HostEffectLedgerError> {
    validate_name(name)?;
    let name = CString::new(name).map_err(|_| invalid_record())?;
    let mut value = std::mem::MaybeUninit::<libc::stat>::zeroed();
    let result = unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            name.as_ptr(),
            value.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ENOENT) {
            return Ok(None);
        }
        return Err(ledger_io());
    }
    let value = unsafe { value.assume_init() };
    if value.st_size < 0 {
        return Err(tampered());
    }
    Ok(Some(FileIdentity {
        device: value.st_dev as u64,
        inode: value.st_ino as u64,
        mode: value.st_mode as u32,
        hard_links: value.st_nlink as u64,
        length: value.st_size as u64,
    }))
}

fn rename_relative(
    directory: &File,
    source: &str,
    target: &str,
) -> Result<(), HostEffectLedgerError> {
    validate_name(source)?;
    validate_name(target)?;
    let source = CString::new(source).map_err(|_| invalid_record())?;
    let target = CString::new(target).map_err(|_| invalid_record())?;
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

fn unlink_relative(directory: &File, name: &str) -> Result<(), HostEffectLedgerError> {
    validate_name(name)?;
    let name = CString::new(name).map_err(|_| invalid_record())?;
    if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
        return Err(ledger_io());
    }
    Ok(())
}

fn validate_name(value: &str) -> Result<(), HostEffectLedgerError> {
    if value.is_empty()
        || value.len() > 160
        || value.as_bytes().contains(&0)
        || value.contains('/')
        || value == "."
        || value == ".."
    {
        return Err(invalid_record());
    }
    Ok(())
}

fn validate_id(value: &str) -> Result<(), HostEffectLedgerError> {
    if !valid_id(value) {
        return Err(invalid_record());
    }
    Ok(())
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn invalid_record() -> HostEffectLedgerError {
    HostEffectLedgerError::new(HostEffectLedgerErrorId::InvalidRecord)
}

fn invalid_transition() -> HostEffectLedgerError {
    HostEffectLedgerError::new(HostEffectLedgerErrorId::InvalidTransition)
}

fn stale_head() -> HostEffectLedgerError {
    HostEffectLedgerError::new(HostEffectLedgerErrorId::StaleHead)
}

fn replay_error() -> HostEffectLedgerError {
    HostEffectLedgerError::new(HostEffectLedgerErrorId::Replay)
}

fn tampered() -> HostEffectLedgerError {
    HostEffectLedgerError::new(HostEffectLedgerErrorId::Tampered)
}

fn ledger_io() -> HostEffectLedgerError {
    HostEffectLedgerError::new(HostEffectLedgerErrorId::Io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::fs::symlink;
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;

    fn d(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    fn reservation(
        head: &HostEffectLedgerHead,
        permit: char,
        nonce: char,
        semantic: char,
    ) -> HostEffectReservation {
        HostEffectReservation {
            issuer_id: "root-actor".to_owned(),
            ledger_id: "host-ledger".to_owned(),
            key_id: d('a'),
            permit_id: d(permit),
            semantic_key_sha256: d(semantic),
            nonce_sha256: d(nonce),
            binding_sha256: d('b'),
            expected_head_sha256: head.head_sha256().to_owned(),
            issued_at_unix_ms: 1_000,
            expires_at_unix_ms: 2_000,
        }
    }

    #[test]
    fn durable_ledger_reopens_and_enforces_the_exact_state_graph() {
        let fixture = LedgerFixture::new();
        let ledger = FileHostEffectLedger::create(&fixture.root, "host-ledger".to_owned()).unwrap();
        let initial = ledger.head().unwrap();
        assert_eq!(initial.generation(), 0);
        let reserved = ledger
            .reserve(reservation(&initial, '1', '2', '3'))
            .unwrap();
        assert_eq!(reserved.state(), HostEffectState::Reserved);

        let head = ledger.head().unwrap();
        let in_flight = ledger
            .transition(
                HostEffectTransition::new(
                    d('1'),
                    HostEffectState::Reserved,
                    HostEffectState::InFlight,
                    head,
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(in_flight.state(), HostEffectState::InFlight);

        let head = ledger.head().unwrap();
        let ambiguous = ledger
            .transition(
                HostEffectTransition::new(
                    d('1'),
                    HostEffectState::InFlight,
                    HostEffectState::Ambiguous,
                    head,
                    Some(d('4')),
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(ambiguous.state(), HostEffectState::Ambiguous);

        let reopened = FileHostEffectLedger::open(&fixture.root, "host-ledger".to_owned()).unwrap();
        assert_eq!(
            reopened.read(&d('1')).unwrap().unwrap().state(),
            HostEffectState::Ambiguous
        );
        let settled = reopened
            .transition(
                HostEffectTransition::new(
                    d('1'),
                    HostEffectState::Ambiguous,
                    HostEffectState::Settled,
                    reopened.head().unwrap(),
                    Some(d('5')),
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(settled.state(), HostEffectState::Settled);
        assert_eq!(reopened.head().unwrap().generation(), 4);
    }

    #[test]
    fn duplicate_nonce_semantic_key_and_stale_head_fail_closed() {
        let fixture = LedgerFixture::new();
        let ledger = FileHostEffectLedger::create(&fixture.root, "host-ledger".to_owned()).unwrap();
        let initial = ledger.head().unwrap();
        ledger
            .reserve(reservation(&initial, '1', '2', '3'))
            .unwrap();

        assert_eq!(
            ledger
                .reserve(reservation(&initial, '4', '5', '6'))
                .unwrap_err()
                .id(),
            HostEffectLedgerErrorId::StaleHead
        );
        let current = ledger.head().unwrap();
        assert_eq!(
            ledger
                .reserve(reservation(&current, '4', '2', '6'))
                .unwrap_err()
                .id(),
            HostEffectLedgerErrorId::Replay
        );
        assert_eq!(
            ledger
                .reserve(reservation(&current, '4', '5', '3'))
                .unwrap_err()
                .id(),
            HostEffectLedgerErrorId::Replay
        );
    }

    #[test]
    fn concurrent_reservation_has_one_durable_winner() {
        let fixture = LedgerFixture::new();
        let ledger = Arc::new(
            FileHostEffectLedger::create(&fixture.root, "host-ledger".to_owned()).unwrap(),
        );
        let head = ledger.head().unwrap();
        let mut workers = Vec::new();
        for index in 0..16_u8 {
            let ledger = Arc::clone(&ledger);
            let head = head.clone();
            workers.push(thread::spawn(move || {
                let permit = char::from(b'a' + index);
                let nonce = char::from(b'a' + index);
                ledger.reserve(reservation(&head, permit, nonce, '1'))
            }));
        }
        let outcomes = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(outcomes.iter().filter(|row| row.is_ok()).count(), 1);
        assert_eq!(ledger.head().unwrap().generation(), 1);
        let state_bytes = fs::read(fixture.root.join(STATE_NAME)).unwrap();
        let reopened = FileHostEffectLedger::open(&fixture.root, "host-ledger".to_owned()).unwrap();
        assert_eq!(reopened.head().unwrap().generation(), 1);
        assert_eq!(
            fs::read(fixture.root.join(STATE_NAME)).unwrap(),
            state_bytes
        );
    }

    #[test]
    fn authenticated_state_rejects_tamper_and_same_session_rollback() {
        let fixture = LedgerFixture::new();
        let ledger = FileHostEffectLedger::create(&fixture.root, "host-ledger".to_owned()).unwrap();
        let initial_bytes = fs::read(fixture.root.join(STATE_NAME)).unwrap();
        let initial = ledger.head().unwrap();
        ledger
            .reserve(reservation(&initial, '1', '2', '3'))
            .unwrap();

        overwrite(&fixture.root.join(STATE_NAME), &initial_bytes);
        assert_eq!(
            ledger.head().unwrap_err().id(),
            HostEffectLedgerErrorId::Tampered
        );

        let mut tampered = initial_bytes;
        let index = tampered.iter().position(|byte| *byte == b'{').unwrap();
        tampered[index] = b'[';
        overwrite(&fixture.root.join(STATE_NAME), &tampered);
        let opened = FileHostEffectLedger::open(&fixture.root, "host-ledger".to_owned());
        assert!(matches!(
            opened,
            Err(error) if error.id() == HostEffectLedgerErrorId::Tampered
        ));
    }

    #[test]
    fn read_and_head_paths_leave_ledger_bytes_and_modes_unchanged() {
        let fixture = LedgerFixture::new();
        let ledger = FileHostEffectLedger::create(&fixture.root, "host-ledger".to_owned()).unwrap();
        let before = fixture.snapshot();
        let _ = ledger.head().unwrap();
        assert!(ledger.read(&d('1')).unwrap().is_none());
        let after = fixture.snapshot();
        assert_eq!(before, after);
    }

    #[test]
    fn ledger_and_key_identity_substitution_fail_closed() {
        let fixture = LedgerFixture::new();
        let ledger = FileHostEffectLedger::create(&fixture.root, "host-ledger".to_owned()).unwrap();
        let wrong = FileHostEffectLedger::open(&fixture.root, "other-ledger".to_owned());
        assert!(matches!(
            wrong,
            Err(error) if error.id() == HostEffectLedgerErrorId::Tampered
        ));

        let key = fixture.root.join(KEY_NAME);
        fs::rename(&key, fixture.root.join("held.key")).unwrap();
        let mut replacement = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&key)
            .unwrap();
        replacement.write_all(&[7_u8; KEY_BYTES]).unwrap();
        replacement.sync_all().unwrap();
        assert_eq!(
            ledger.head().unwrap_err().id(),
            HostEffectLedgerErrorId::Tampered
        );
    }

    #[test]
    fn key_replacement_between_identity_and_read_cannot_splice_state() {
        let fixture = LedgerFixture::new();
        let ledger = Arc::new(
            FileHostEffectLedger::create(&fixture.root, "host-ledger".to_owned()).unwrap(),
        );
        let reached = Arc::new(std::sync::Barrier::new(2));
        let release = Arc::new(std::sync::Barrier::new(2));
        ledger
            .install_key_open_hook(KeyOpenHook {
                reached: Arc::clone(&reached),
                release: Arc::clone(&release),
            })
            .unwrap();

        let contender = Arc::clone(&ledger);
        let worker = thread::spawn(move || contender.head());
        reached.wait();

        let named_key = fixture.root.join(KEY_NAME);
        let held_key = fixture.root.with_extension("held-key");
        fs::rename(&named_key, &held_key).unwrap();
        let replacement_bytes = [7_u8; KEY_BYTES];
        let mut replacement = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&named_key)
            .unwrap();
        replacement.write_all(&replacement_bytes).unwrap();
        replacement.sync_all().unwrap();

        let replacement_key = LedgerKey(replacement_bytes);
        let payload = initial_payload(
            "host-ledger",
            &digest(&replacement_key.0),
            ledger.lock_identity,
        )
        .unwrap();
        let replacement_state = encode_snapshot(&payload, &replacement_key).unwrap();
        overwrite(&fixture.root.join(STATE_NAME), &replacement_state);
        release.wait();

        assert_eq!(
            worker.join().unwrap().unwrap_err().id(),
            HostEffectLedgerErrorId::Tampered
        );
        assert_eq!(fs::read(named_key).unwrap(), replacement_bytes);
        assert_eq!(
            fs::read(fixture.root.join(STATE_NAME)).unwrap(),
            replacement_state
        );
    }

    #[test]
    fn lock_replacement_after_prelock_check_cannot_split_serialization() {
        let fixture = LedgerFixture::new();
        let ledger = Arc::new(
            FileHostEffectLedger::create(&fixture.root, "host-ledger".to_owned()).unwrap(),
        );
        let state_before = fs::read(fixture.root.join(STATE_NAME)).unwrap();
        let reached = Arc::new(std::sync::Barrier::new(2));
        let release = Arc::new(std::sync::Barrier::new(2));
        ledger
            .install_lock_open_hook(LockOpenHook {
                reached: Arc::clone(&reached),
                release: Arc::clone(&release),
            })
            .unwrap();

        let contender = Arc::clone(&ledger);
        let worker = thread::spawn(move || contender.head());
        reached.wait();

        let named_lock = fixture.root.join(LOCK_NAME);
        let held_lock = fixture.root.with_extension("held-lock");
        fs::rename(&named_lock, &held_lock).unwrap();
        let replacement = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&named_lock)
            .unwrap();
        replacement.sync_all().unwrap();
        release.wait();

        assert_eq!(
            worker.join().unwrap().unwrap_err().id(),
            HostEffectLedgerErrorId::Tampered
        );
        assert_eq!(
            fs::read(fixture.root.join(STATE_NAME)).unwrap(),
            state_before
        );
        let reopened = FileHostEffectLedger::open(&fixture.root, "host-ledger".to_owned());
        assert!(matches!(
            reopened,
            Err(error) if error.id() == HostEffectLedgerErrorId::Tampered
        ));
        assert_eq!(
            fs::read(fixture.root.join(STATE_NAME)).unwrap(),
            state_before
        );
    }

    #[test]
    fn unknown_and_special_ledger_entries_fail_closed_without_cleanup() {
        let fixture = LedgerFixture::new();
        let ledger = FileHostEffectLedger::create(&fixture.root, "host-ledger".to_owned()).unwrap();
        let unknown = fixture.root.join("unknown.json");
        fs::write(&unknown, b"{}\n").unwrap();
        fs::set_permissions(&unknown, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            ledger.head().unwrap_err().id(),
            HostEffectLedgerErrorId::Tampered
        );
        assert!(unknown.exists());
        fs::remove_file(&unknown).unwrap();

        let state = fixture.root.join(STATE_NAME);
        let held = fixture.root.join("held-state");
        fs::rename(&state, &held).unwrap();
        symlink(&held, &state).unwrap();
        assert_eq!(
            ledger.head().unwrap_err().id(),
            HostEffectLedgerErrorId::Tampered
        );
        assert!(
            fs::symlink_metadata(&state)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn separate_process_reservation_race_has_one_winner() {
        let fixture = LedgerFixture::new();
        let ledger = FileHostEffectLedger::create(&fixture.root, "host-ledger".to_owned()).unwrap();
        let initial = ledger.head().unwrap();
        let executable = std::env::current_exe().unwrap();
        let hex = b"0123456789abcdef";
        let barrier = fixture.root.with_extension("process-barrier");
        fs::DirBuilder::new().mode(0o700).create(&barrier).unwrap();
        let mut children = Vec::new();
        for value in hex {
            let child = Command::new(&executable)
                .arg("--ignored")
                .arg("--exact")
                .arg("distribution::host_effect::ledger::tests::subprocess_reserve_helper")
                .arg("--nocapture")
                .env("HUL_LEDGER_CHILD_ROOT", &fixture.root)
                .env("HUL_LEDGER_CHILD_HEAD", initial.head_sha256())
                .env(
                    "HUL_LEDGER_CHILD_GENERATION",
                    initial.generation().to_string(),
                )
                .env("HUL_LEDGER_CHILD_VALUE", char::from(*value).to_string())
                .env("HUL_LEDGER_CHILD_BARRIER", &barrier)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            children.push(child);
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        loop {
            let ready = fs::read_dir(&barrier)
                .unwrap()
                .map(|entry| entry.unwrap().file_name().into_string().unwrap())
                .filter(|name| name.starts_with("ready-"))
                .collect::<BTreeSet<_>>();
            if ready.len() == hex.len() {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "only {} of {} child processes reached the pre-flock barrier",
                ready.len(),
                hex.len()
            );
            thread::sleep(std::time::Duration::from_millis(1));
        }
        let release = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(barrier.join("release"))
            .unwrap();
        release.sync_all().unwrap();
        let outputs = children
            .into_iter()
            .map(|child| child.wait_with_output().unwrap())
            .collect::<Vec<_>>();
        for output in &outputs {
            assert!(
                output.status.success(),
                "child failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let winners = outputs
            .iter()
            .filter(|output| {
                String::from_utf8_lossy(&output.stdout).contains("HUL_LEDGER_CHILD_RESULT=winner")
            })
            .count();
        assert_eq!(winners, 1);
        assert_eq!(ledger.head().unwrap().generation(), 1);
    }

    #[test]
    #[ignore = "subprocess helper invoked only by separate_process_reservation_race_has_one_winner"]
    fn subprocess_reserve_helper() {
        let root = PathBuf::from(std::env::var_os("HUL_LEDGER_CHILD_ROOT").unwrap());
        let head_sha256 = std::env::var("HUL_LEDGER_CHILD_HEAD").unwrap();
        let generation = std::env::var("HUL_LEDGER_CHILD_GENERATION")
            .unwrap()
            .parse::<u64>()
            .unwrap();
        let value = std::env::var("HUL_LEDGER_CHILD_VALUE")
            .unwrap()
            .chars()
            .next()
            .unwrap();
        let ledger = FileHostEffectLedger::open(&root, "host-ledger".to_owned()).unwrap();
        let head = HostEffectLedgerHead::new(generation, head_sha256).unwrap();
        match ledger.reserve(reservation(&head, value, value, 'f')) {
            Ok(_) => println!("HUL_LEDGER_CHILD_RESULT=winner"),
            Err(error)
                if matches!(
                    error.id(),
                    HostEffectLedgerErrorId::StaleHead | HostEffectLedgerErrorId::Replay
                ) =>
            {
                println!("HUL_LEDGER_CHILD_RESULT=refused")
            }
            Err(error) => panic!("unexpected child ledger error: {:?}", error.id()),
        }
    }

    fn overwrite(path: &Path, bytes: &[u8]) {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(path)
            .unwrap();
        file.write_all(bytes).unwrap();
        file.sync_all().unwrap();
    }

    struct LedgerFixture {
        root: PathBuf,
    }

    impl LedgerFixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let base = std::env::temp_dir();
            loop {
                let next = NEXT.fetch_add(1, Ordering::Relaxed);
                let root = base.join(format!(
                    "hul-host-effect-ledger-{}-{next}",
                    std::process::id()
                ));
                match fs::DirBuilder::new().mode(0o700).create(&root) {
                    Ok(()) => return Self { root },
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("create ledger fixture: {error}"),
                }
            }
        }

        fn snapshot(&self) -> Vec<(String, u32, Vec<u8>)> {
            let mut rows = fs::read_dir(&self.root)
                .unwrap()
                .map(|entry| {
                    let entry = entry.unwrap();
                    let metadata = entry.metadata().unwrap();
                    (
                        entry.file_name().into_string().unwrap(),
                        metadata.permissions().mode() & 0o777,
                        fs::read(entry.path()).unwrap(),
                    )
                })
                .collect::<Vec<_>>();
            rows.sort_by(|left, right| left.0.cmp(&right.0));
            rows
        }
    }

    impl Drop for LedgerFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
            let _ = fs::remove_file(self.root.with_extension("held-lock"));
            let _ = fs::remove_file(self.root.with_extension("held-key"));
            let _ = fs::remove_dir_all(self.root.with_extension("process-barrier"));
        }
    }
}
