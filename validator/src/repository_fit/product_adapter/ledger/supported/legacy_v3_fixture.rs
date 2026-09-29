use super::*;

#[cfg(test)]
pub(crate) fn write_legacy_v3_device_number_for_test(
    root: &Path,
    replacement_device: u64,
) -> Result<(), LedgerError> {
    simulate_legacy_identity_for_test(root, replacement_device, None)
}

#[cfg(test)]
pub(crate) fn simulate_durable_identity_drift_for_test(
    root: &Path,
    replacement_inode: u64,
) -> Result<(), LedgerError> {
    let current_device = fs::metadata(root).map_err(|_| ledger_io())?.dev();
    simulate_legacy_identity_for_test(root, current_device, Some(replacement_inode))
}

#[cfg(test)]
fn simulate_legacy_identity_for_test(
    root: &Path,
    replacement_device: u64,
    replacement_inode: Option<u64>,
) -> Result<(), LedgerError> {
    let store = Store::open(root)?;
    if replacement_device == 0
        || (replacement_device == store.root_identity.device && replacement_inode.is_none())
        || replacement_inode.is_some_and(|inode| inode == store.root_identity.inode)
    {
        return Err(invalid_store());
    }
    let key_file = store.open_existing(KEY_NAME, libc::O_RDONLY)?;
    let key = read_key(&key_file)?;
    let state_file = store.open_existing(STATE_NAME, libc::O_RDONLY)?;
    let bytes = read_bounded(&state_file, MAX_LEDGER_BYTES)?;
    let current: SnapshotEnvelope = serde_json::from_slice(&bytes).map_err(|_| tampered())?;
    if current.schema_version != ENVELOPE_SCHEMA || current.payload.schema_version != LEDGER_SCHEMA
    {
        return Err(tampered());
    }
    let mut payload = LegacySnapshotPayloadV3 {
        schema_version: LEGACY_LEDGER_SCHEMA.to_owned(),
        store_id: current.payload.store_id,
        authority_id: current.payload.authority_id,
        key_id: current.payload.key_id,
        root_identity: RootIdentity {
            device: replacement_device,
            inode: replacement_inode.unwrap_or(current.payload.root_identity.inode),
            uid: current.payload.root_identity.uid,
            gid: current.payload.root_identity.gid,
            mode: current.payload.root_identity.mode,
        },
        lock_identity: FileIdentity {
            device: replacement_device,
            inode: current.payload.lock_identity.inode,
            links: current.payload.lock_identity.links,
            uid: current.payload.lock_identity.uid,
            gid: current.payload.lock_identity.gid,
            mode: current.payload.lock_identity.mode,
            length: current.payload.lock_identity.length,
            changed_seconds: current.payload.lock_identity.changed_seconds,
            changed_nanoseconds: current.payload.lock_identity.changed_nanoseconds,
        },
        generation: current.payload.generation,
        head_sha256: current.payload.head_sha256,
        events: current.payload.events,
    };
    let mut head = digest(
        &serde_json::to_vec(&(
            LEGACY_INITIAL_HEAD_DOMAIN,
            &payload.store_id,
            &payload.authority_id,
            &payload.key_id,
            payload.root_identity,
            payload.lock_identity,
        ))
        .map_err(|_| tampered())?,
    );
    for event in &mut payload.events {
        event.prior_head_sha256.clone_from(&head);
        event.event_sha256 = event_digest(event)?;
        head.clone_from(&event.event_sha256);
    }
    payload.head_sha256 = head;
    let canonical = serde_json::to_vec(&payload).map_err(|_| invalid_transition())?;
    let mut mac = HmacSha256::new_from_slice(&key.0).map_err(|_| invalid_transition())?;
    mac.update(&canonical);
    let replacement = serde_json::to_vec(&LegacySnapshotEnvelopeV3 {
        schema_version: LEGACY_ENVELOPE_SCHEMA.to_owned(),
        payload,
        hmac_sha256: format!("sha256:{:x}", mac.finalize().into_bytes()),
    })
    .map_err(|_| invalid_transition())?;
    fs::write(root.join(STATE_NAME), replacement).map_err(|_| ledger_io())
}
