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
