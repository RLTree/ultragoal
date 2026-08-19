use super::*;

pub(crate) fn decode_snapshot(
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
    let schema = serde_json::from_slice::<serde_json::Value>(bytes)
        .map_err(|_| tampered())?
        .get("schema_version")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(tampered)?
        .to_owned();
    if schema == LEGACY_ENVELOPE_SCHEMA {
        return decode_legacy_snapshot(
            bytes,
            key,
            store_id,
            authority_id,
            key_id,
            root_identity,
            lock_identity,
        );
    }
    let envelope: SnapshotEnvelope = serde_json::from_slice(bytes).map_err(|_| tampered())?;
    if envelope.schema_version != ENVELOPE_SCHEMA
        || envelope.payload.schema_version != LEDGER_SCHEMA
        || envelope.payload.store_id != store_id
        || envelope.payload.authority_id != authority_id
        || envelope.payload.key_id != key_id
        || envelope.payload.root_identity != root_identity.durable()
        || envelope.payload.lock_identity != lock_identity.durable()
        || !valid_digest(&envelope.payload.chain_origin_sha256)
        || !envelope
            .payload
            .predecessor_envelope_sha256
            .as_deref()
            .is_none_or(valid_digest)
        || !valid_digest(&envelope.hmac_sha256)
    {
        return Err(tampered());
    }
    authenticate(&envelope.payload, &envelope.hmac_sha256, key)?;
    Ok(envelope.payload)
}

fn decode_legacy_snapshot(
    bytes: &[u8],
    key: &LedgerKey,
    store_id: &str,
    authority_id: &str,
    key_id: &str,
    root_identity: RootIdentity,
    lock_identity: FileIdentity,
) -> Result<SnapshotPayload, LedgerError> {
    let envelope: LegacySnapshotEnvelopeV3 =
        serde_json::from_slice(bytes).map_err(|_| tampered())?;
    if envelope.schema_version != LEGACY_ENVELOPE_SCHEMA
        || envelope.payload.schema_version != LEGACY_LEDGER_SCHEMA
        || envelope.payload.store_id != store_id
        || envelope.payload.authority_id != authority_id
        || envelope.payload.key_id != key_id
        || envelope.payload.root_identity.durable() != root_identity.durable()
        || envelope.payload.lock_identity.durable() != lock_identity.durable()
        || envelope.payload.root_identity.device == 0
        || envelope.payload.lock_identity.device != envelope.payload.root_identity.device
        || !valid_digest(&envelope.hmac_sha256)
    {
        return Err(tampered());
    }
    authenticate(&envelope.payload, &envelope.hmac_sha256, key)?;
    let chain_origin_sha256 = digest(
        &serde_json::to_vec(&(
            LEGACY_INITIAL_HEAD_DOMAIN,
            &envelope.payload.store_id,
            &envelope.payload.authority_id,
            &envelope.payload.key_id,
            envelope.payload.root_identity,
            envelope.payload.lock_identity,
        ))
        .map_err(|_| tampered())?,
    );
    let payload = SnapshotPayload {
        schema_version: LEDGER_SCHEMA.to_owned(),
        store_id: envelope.payload.store_id,
        authority_id: envelope.payload.authority_id,
        key_id: envelope.payload.key_id,
        root_identity: root_identity.durable(),
        lock_identity: lock_identity.durable(),
        chain_origin_sha256,
        predecessor_envelope_sha256: Some(digest(bytes)),
        generation: envelope.payload.generation,
        head_sha256: envelope.payload.head_sha256,
        events: envelope.payload.events,
    };
    replay(&payload)?;
    Ok(payload)
}

fn authenticate(
    payload: &impl Serialize,
    supplied: &str,
    key: &LedgerKey,
) -> Result<(), LedgerError> {
    let canonical = serde_json::to_vec(payload).map_err(|_| tampered())?;
    let mut mac = HmacSha256::new_from_slice(&key.0).map_err(|_| tampered())?;
    mac.update(&canonical);
    let supplied = decode_digest(supplied)?;
    mac.verify_slice(&supplied).map_err(|_| tampered())?;
    Ok(())
}

pub(crate) fn encode_snapshot(
    payload: &SnapshotPayload,
    key: &LedgerKey,
) -> Result<Vec<u8>, LedgerError> {
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

pub(crate) fn replay(payload: &SnapshotPayload) -> Result<ReplayState, LedgerError> {
    if payload.events.len() > MAX_EVENTS
        || payload.generation != payload.events.len() as u64
        || !valid_digest(&payload.head_sha256)
    {
        return Err(tampered());
    }
    let mut state = ReplayState::default();
    let fresh_origin = digest(
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
    if payload.predecessor_envelope_sha256.is_none() && payload.chain_origin_sha256 != fresh_origin
    {
        return Err(tampered());
    }
    let mut head = payload.chain_origin_sha256.clone();
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
            RepositoryFitLedgerState::EffectStarted => {
                let prior = state
                    .records
                    .get(&event.reservation_id)
                    .ok_or_else(tampered)?;
                if prior.state != RepositoryFitLedgerState::Reserved
                    || !same_reservation(prior, event)
                    || event.transition_tick < prior.transition_tick
                    || state.active_targets.get(&event.target_scope_id)
                        != Some(&event.reservation_id)
                {
                    return Err(tampered());
                }
            }
            terminal => {
                if !terminal.terminal() {
                    return Err(tampered());
                }
                let prior = state
                    .records
                    .get(&event.reservation_id)
                    .ok_or_else(tampered)?;
                if !matches!(
                    prior.state,
                    RepositoryFitLedgerState::Reserved | RepositoryFitLedgerState::EffectStarted
                ) || !same_reservation(prior, event)
                    || event.transition_tick < prior.transition_tick
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

pub(crate) fn valid_event(event: &LedgerEvent) -> bool {
    valid_digest(&event.event_id)
        && valid_digest(&event.prior_head_sha256)
        && valid_digest(&event.reservation_id)
        && valid_digest(&event.binding_sha256)
        && valid_digest(&event.semantic_effect_id)
        && valid_digest(&event.target_scope_id)
        && valid_digest(&event.permit_id)
        && valid_digest(&event.nonce_sha256)
        && valid_digest(&event.recovery_intent_sha256)
        && recovery_intent_matches(&event.recovery, &event.recovery_intent_sha256)
        && event.expires_tick >= event.issued_tick
        && valid_recovery(&event.recovery)
        && event.transition_tick >= event.issued_tick
        && match event.state {
            RepositoryFitLedgerState::Reserved => {
                event.transition_tick == event.issued_tick
                    && event.terminal_sha256.is_none()
                    && event.error_id.is_none()
            }
            RepositoryFitLedgerState::EffectStarted => {
                event.transition_tick <= event.expires_tick
                    && event.terminal_sha256.is_none()
                    && event.error_id.is_none()
            }
            RepositoryFitLedgerState::Committed => {
                event.terminal_sha256.as_deref().is_some_and(valid_digest)
                    && event.error_id.is_none()
            }
            _ => event.terminal_sha256.as_deref().is_some_and(valid_digest),
        }
}
