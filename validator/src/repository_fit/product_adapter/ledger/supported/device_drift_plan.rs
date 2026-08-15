use super::super::{
    RepositoryFitAuthorityDeviceDrift, RepositoryFitStoredLockIdentity,
    RepositoryFitStoredRootIdentity,
};
use super::*;

pub(crate) fn assess_device_drift(
    root: &Path,
    store_id: &str,
) -> Result<Option<RepositoryFitAuthorityDeviceDrift>, LedgerError> {
    if !valid_digest(store_id) {
        return Err(invalid_store());
    }
    let store = Store::open(root)?;
    store.verify_root()?;
    if store.names()?
        != BTreeSet::from([
            KEY_NAME.to_owned(),
            LOCK_NAME.to_owned(),
            STATE_NAME.to_owned(),
        ])
    {
        return Err(tampered());
    }

    let lock = store.open_existing(LOCK_NAME, libc::O_RDWR)?;
    let current_lock = exact_identity(&store, LOCK_NAME, &lock, 0o600)?;
    let guard = ProcessLock::acquire(lock)?;
    store.verify_root()?;
    let locked_identity = exact_identity(&store, LOCK_NAME, &guard.0, 0o600)?;
    if current_lock != locked_identity
        || read_bounded(&guard.0, LOCK_MARKER.len() as u64)? != LOCK_MARKER
    {
        return Err(tampered());
    }

    let key_file = store.open_existing(KEY_NAME, libc::O_RDONLY)?;
    let key_identity = exact_identity(&store, KEY_NAME, &key_file, 0o600)?;
    let key = read_key(&key_file)?;
    let key_id = digest(&key.0);
    let authority_id = authority_id(store_id, &key_id)?;

    let state_file = store.open_existing(STATE_NAME, libc::O_RDONLY)?;
    let state_identity = exact_identity(&store, STATE_NAME, &state_file, 0o600)?;
    let ledger_bytes = read_bounded(&state_file, MAX_LEDGER_BYTES)?;
    let envelope: SnapshotEnvelope =
        serde_json::from_slice(&ledger_bytes).map_err(|_| tampered())?;
    validate_envelope_identity(&envelope, store_id, &authority_id, &key_id)?;
    authenticate_payload(&envelope, &key)?;
    let replayed = replay(&envelope.payload)?;

    let stored_root = envelope.payload.root_identity;
    let stored_lock = envelope.payload.lock_identity;
    let current_root = store.root_identity;
    if stored_root == current_root && stored_lock == current_lock {
        return Ok(None);
    }
    if !root_differs_only_by_device(stored_root, current_root)
        || !lock_differs_only_by_device(stored_lock, current_lock)
        || stored_root.device == 0
        || current_root.device == 0
        || stored_root.device == current_root.device
        || stored_lock.device != stored_root.device
        || current_lock.device != current_root.device
    {
        return Err(tampered());
    }

    store.verify_root()?;
    if exact_identity(&store, KEY_NAME, &key_file, 0o600)? != key_identity
        || exact_identity(&store, LOCK_NAME, &guard.0, 0o600)? != current_lock
        || exact_identity(&store, STATE_NAME, &state_file, 0o600)? != state_identity
        || read_bounded(&state_file, MAX_LEDGER_BYTES)? != ledger_bytes
    {
        return Err(tampered());
    }

    let nonterminal_reservation_count = replayed
        .records
        .values()
        .filter(|event| !event.state.terminal())
        .count();
    let authority_inventory_sha256 = digest(
        &serde_json::to_vec(&(
            "repository-fit-authority-device-drift-inventory-v1",
            store_id,
            &authority_id,
            &key_id,
            current_root,
            current_lock,
            key_identity,
            state_identity,
            digest(&ledger_bytes),
        ))
        .map_err(|_| tampered())?,
    );
    Ok(Some(RepositoryFitAuthorityDeviceDrift {
        authority_inventory_sha256,
        ledger_sha256: digest(&ledger_bytes),
        authority_id,
        generation: envelope.payload.generation,
        head_sha256: envelope.payload.head_sha256.clone(),
        event_count: envelope.payload.events.len(),
        reservation_count: replayed.records.len(),
        nonterminal_reservation_count,
        stored_root: stored_root.into(),
        current_root: current_root.into(),
        stored_lock: stored_lock.into(),
        current_lock: current_lock.into(),
    }))
}

fn validate_envelope_identity(
    envelope: &SnapshotEnvelope,
    store_id: &str,
    authority_id: &str,
    key_id: &str,
) -> Result<(), LedgerError> {
    if envelope.schema_version != ENVELOPE_SCHEMA
        || envelope.payload.schema_version != LEDGER_SCHEMA
        || envelope.payload.store_id != store_id
        || envelope.payload.authority_id != authority_id
        || envelope.payload.key_id != key_id
        || !valid_digest(&envelope.hmac_sha256)
    {
        return Err(tampered());
    }
    Ok(())
}

fn authenticate_payload(envelope: &SnapshotEnvelope, key: &LedgerKey) -> Result<(), LedgerError> {
    let canonical = serde_json::to_vec(&envelope.payload).map_err(|_| tampered())?;
    let mut mac = HmacSha256::new_from_slice(&key.0).map_err(|_| tampered())?;
    mac.update(&canonical);
    let supplied = decode_digest(&envelope.hmac_sha256)?;
    mac.verify_slice(&supplied).map_err(|_| tampered())
}

fn root_differs_only_by_device(left: RootIdentity, right: RootIdentity) -> bool {
    left.inode == right.inode
        && left.uid == right.uid
        && left.gid == right.gid
        && left.mode == right.mode
}

fn lock_differs_only_by_device(left: FileIdentity, right: FileIdentity) -> bool {
    left.inode == right.inode
        && left.links == right.links
        && left.uid == right.uid
        && left.gid == right.gid
        && left.mode == right.mode
        && left.length == right.length
        && left.changed_seconds == right.changed_seconds
        && left.changed_nanoseconds == right.changed_nanoseconds
}

impl From<RootIdentity> for RepositoryFitStoredRootIdentity {
    fn from(value: RootIdentity) -> Self {
        Self {
            device: value.device,
            inode: value.inode,
            uid: value.uid,
            gid: value.gid,
            mode: value.mode,
        }
    }
}

impl From<FileIdentity> for RepositoryFitStoredLockIdentity {
    fn from(value: FileIdentity) -> Self {
        Self {
            device: value.device,
            inode: value.inode,
            links: value.links,
            uid: value.uid,
            gid: value.gid,
            mode: value.mode,
            length: value.length,
            changed_seconds: value.changed_seconds,
            changed_nanoseconds: value.changed_nanoseconds,
        }
    }
}

#[cfg(test)]
pub(crate) fn simulate_device_drift_for_test(
    root: &Path,
    replacement_device: u64,
) -> Result<(), LedgerError> {
    let store = Store::open(root)?;
    if replacement_device == 0 || replacement_device == store.root_identity.device {
        return Err(invalid_store());
    }
    let key_file = store.open_existing(KEY_NAME, libc::O_RDONLY)?;
    let key = read_key(&key_file)?;
    let state_file = store.open_existing(STATE_NAME, libc::O_RDONLY)?;
    let bytes = read_bounded(&state_file, MAX_LEDGER_BYTES)?;
    let mut envelope: SnapshotEnvelope = serde_json::from_slice(&bytes).map_err(|_| tampered())?;
    envelope.payload.root_identity.device = replacement_device;
    envelope.payload.lock_identity.device = replacement_device;
    let mut head = digest(
        &serde_json::to_vec(&(
            INITIAL_HEAD_DOMAIN,
            &envelope.payload.store_id,
            &envelope.payload.authority_id,
            &envelope.payload.key_id,
            envelope.payload.root_identity,
            envelope.payload.lock_identity,
        ))
        .map_err(|_| tampered())?,
    );
    for event in &mut envelope.payload.events {
        event.prior_head_sha256.clone_from(&head);
        event.event_sha256 = event_digest(event)?;
        head.clone_from(&event.event_sha256);
    }
    envelope.payload.head_sha256 = head;
    let replacement = encode_snapshot(&envelope.payload, &key)?;
    fs::write(root.join(STATE_NAME), replacement).map_err(|_| ledger_io())
}
