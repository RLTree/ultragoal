use super::*;

#[path = "payload_validation.rs"]
mod payload_validation;

pub(super) use payload_validation::{validate_binding, validate_payload, validate_token};

pub(super) fn initial_payload(
    authority_id: &str,
    key_id: &str,
    root_identity: RootIdentity,
    lock_identity: FileIdentity,
) -> Result<Payload, RoutineError> {
    Ok(Payload {
        schema_version: SCHEMA.to_owned(),
        authority_id: authority_id.to_owned(),
        key_id: key_id.to_owned(),
        root_identity: root_identity.durable(),
        lock_identity: lock_identity.durable(),
        generation: 0,
        previous_head_sha256: sha256(&canonical(&(
            "routine-production-authority-initial-head-v1",
            authority_id,
        ))?),
        last_tick: now_tick()?,
        attempts: BTreeMap::new(),
        effects: BTreeMap::new(),
        consumed_grants: BTreeSet::new(),
    })
}

pub(super) fn authority_id(
    key_id: &str,
    root: RootIdentity,
    lock: FileIdentity,
) -> Result<String, RoutineError> {
    canonical(&(
        "routine-production-authority-v2",
        key_id,
        root.durable(),
        lock.durable(),
    ))
    .map(|bytes| sha256(&bytes))
}

pub(super) fn legacy_authority_id(
    key_id: &str,
    root: RootIdentity,
    lock: FileIdentity,
) -> Result<String, RoutineError> {
    canonical(&("routine-production-authority-v1", key_id, root, lock)).map(|bytes| sha256(&bytes))
}

pub(super) fn encode(payload: &Payload, key: &LedgerKey) -> Result<Vec<u8>, RoutineError> {
    let payload_bytes = canonical(payload)?;
    let envelope = Envelope {
        payload: payload.clone(),
        hmac_sha256: hmac(key.bytes(), &payload_bytes)?,
    };
    canonical(&envelope)
}

pub(super) fn decode(
    bytes: &[u8],
    key: &LedgerKey,
    authority_id: &str,
    key_id: &str,
    root_identity: RootIdentity,
    lock_identity: FileIdentity,
) -> Result<Payload, RoutineError> {
    let value = serde_json::from_slice::<serde_json::Value>(bytes)
        .map_err(|_| error("routine-production-authority-state-invalid"))?;
    let schema = value
        .get("payload")
        .and_then(|payload| payload.get("schema_version"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| error("routine-production-authority-state-invalid"))?;
    if schema == LEGACY_SCHEMA {
        return decode_legacy(
            bytes,
            key,
            authority_id,
            key_id,
            root_identity,
            lock_identity,
        );
    }
    let envelope: Envelope = serde_json::from_slice(bytes)
        .map_err(|_| error("routine-production-authority-state-invalid"))?;
    if canonical(&envelope)? != bytes
        || envelope.payload.schema_version != SCHEMA
        || envelope.payload.authority_id != authority_id
        || envelope.payload.key_id != key_id
        || envelope.payload.root_identity != root_identity.durable()
        || envelope.payload.lock_identity != lock_identity.durable()
        || envelope.hmac_sha256 != hmac(key.bytes(), &canonical(&envelope.payload)?)?
    {
        return Err(error("routine-production-authority-state-tampered"));
    }
    Ok(envelope.payload)
}

fn decode_legacy(
    bytes: &[u8],
    key: &LedgerKey,
    authority_id: &str,
    key_id: &str,
    root_identity: RootIdentity,
    lock_identity: FileIdentity,
) -> Result<Payload, RoutineError> {
    let envelope: LegacyEnvelopeV4 = serde_json::from_slice(bytes)
        .map_err(|_| error("routine-production-authority-state-invalid"))?;
    if canonical(&envelope)? != bytes
        || envelope.payload.schema_version != LEGACY_SCHEMA
        || envelope.payload.key_id != key_id
        || envelope.payload.authority_id
            != legacy_authority_id(
                key_id,
                envelope.payload.root_identity,
                envelope.payload.lock_identity,
            )?
        || envelope.payload.root_identity.durable() != root_identity.durable()
        || envelope.payload.lock_identity.durable() != lock_identity.durable()
        || envelope.payload.root_identity.device == 0
        || envelope.payload.lock_identity.device != envelope.payload.root_identity.device
        || envelope.hmac_sha256 != hmac(key.bytes(), &canonical(&envelope.payload)?)?
    {
        return Err(error("routine-production-authority-state-tampered"));
    }
    Ok(Payload {
        schema_version: SCHEMA.to_owned(),
        authority_id: authority_id.to_owned(),
        key_id: envelope.payload.key_id,
        root_identity: root_identity.durable(),
        lock_identity: lock_identity.durable(),
        generation: envelope.payload.generation,
        previous_head_sha256: envelope.payload.previous_head_sha256,
        last_tick: envelope.payload.last_tick,
        attempts: envelope.payload.attempts,
        effects: envelope.payload.effects,
        consumed_grants: envelope.payload.consumed_grants,
    })
}
