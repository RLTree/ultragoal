use super::*;

pub(crate) fn initial_payload(
    authority_id: &str,
    key_id: &str,
    root_identity: RootIdentity,
    lock_identity: FileIdentity,
) -> Result<Payload, RoutineError> {
    Ok(Payload {
        schema_version: SCHEMA.to_owned(),
        authority_id: authority_id.to_owned(),
        key_id: key_id.to_owned(),
        root_identity,
        lock_identity,
        generation: 0,
        previous_head_sha256: sha256(&canonical(&(
            "routine-production-authority-initial-head-v1",
            authority_id,
        ))?),
        last_tick: now_tick()?,
        protocols: BTreeMap::new(),
        effects: BTreeMap::new(),
        consumed_grants: BTreeSet::new(),
    })
}

pub(crate) fn authority_id(
    key_id: &str,
    root: RootIdentity,
    lock: FileIdentity,
) -> Result<String, RoutineError> {
    canonical(&("routine-production-authority-v1", key_id, root, lock)).map(|bytes| sha256(&bytes))
}

pub(crate) fn encode(payload: &Payload, key: &LedgerKey) -> Result<Vec<u8>, RoutineError> {
    let payload_bytes = canonical(payload)?;
    let envelope = Envelope {
        payload: payload.clone(),
        hmac_sha256: hmac(&key.0, &payload_bytes)?,
    };
    canonical(&envelope)
}

pub(crate) fn decode(
    bytes: &[u8],
    key: &LedgerKey,
    authority_id: &str,
    key_id: &str,
    root_identity: RootIdentity,
    lock_identity: FileIdentity,
) -> Result<Payload, RoutineError> {
    let envelope: Envelope = serde_json::from_slice(bytes)
        .map_err(|_| error("routine-production-authority-state-invalid"))?;
    if canonical(&envelope)? != bytes
        || envelope.payload.schema_version != SCHEMA
        || envelope.payload.authority_id != authority_id
        || envelope.payload.key_id != key_id
        || envelope.payload.root_identity != root_identity
        || envelope.payload.lock_identity != lock_identity
        || envelope.hmac_sha256 != hmac(&key.0, &canonical(&envelope.payload)?)?
    {
        return Err(error("routine-production-authority-state-tampered"));
    }
    Ok(envelope.payload)
}

pub(crate) fn validate_payload(payload: &Payload) -> Result<(), RoutineError> {
    if !valid(&payload.authority_id)
        || !valid(&payload.key_id)
        || !valid(&payload.previous_head_sha256)
        || payload.protocols.len() > MAX_RECORDS
        || payload.effects.len() > MAX_RECORDS
        || payload.consumed_grants.len() > MAX_RECORDS.saturating_mul(4)
        || payload.protocols.iter().any(|(protocol, record)| {
            protocol != &record.binding.protocol_id
                || validate_binding(&record.binding).is_err()
                || !valid(&record.request_id)
                || !valid(&record.grant_id)
                || !valid(&record.recovery_marker)
                || record
                    .recovery_for
                    .as_ref()
                    .is_some_and(|value| !valid(value))
                || record.issued_tick > record.expires_tick
                || record.expires_tick > record.recovery_deadline_tick
                || record
                    .artifacts
                    .iter()
                    .any(|(digest, witness)| !valid(digest) || !valid(witness))
                || record.failure_evidence.len() > 32
                || record.failure_evidence.iter().any(|evidence| {
                    !evidence.shape_is_valid() || evidence.protocol_id != record.binding.protocol_id
                })
                || validate_output_journal(&record.output_journal).is_err()
        })
        || payload.effects.iter().any(|(effect, protocol)| {
            !valid(effect)
                || !valid(protocol)
                || payload
                    .protocols
                    .get(protocol)
                    .is_none_or(|record| &record.binding.effect_id != effect)
        })
        || payload.consumed_grants.iter().any(|grant| !valid(grant))
    {
        return Err(error("routine-production-authority-state-shape-invalid"));
    }
    Ok(())
}

pub(crate) fn validate_spec(spec: &ReservationSpec) -> Result<(), RoutineError> {
    validate_binding(&spec.binding)?;
    if !valid(&spec.request_id)
        || !valid(&spec.grant_id)
        || !valid(&spec.recovery_marker)
        || spec
            .recovery_for
            .as_ref()
            .is_some_and(|value| !valid(value))
        || spec.reuse_only && spec.recovery_for.is_some()
        || spec.reuse_only != spec.reuse_preauthorization.is_some()
        || validate_output_journal(&spec.output_journal).is_err()
    {
        return Err(error("routine-production-reservation-spec-invalid"));
    }
    Ok(())
}

pub(crate) fn validate_reuse_claims(
    binding: &AuthorityBinding,
    claims: &[ReuseArtifactClaim],
) -> Result<(), RoutineError> {
    let mut intents = BTreeSet::new();
    let mut artifacts = BTreeSet::new();
    if claims.is_empty()
        || claims.iter().any(|claim| {
            claim.protocol_id != binding.protocol_id
                || !valid(&claim.intent_id)
                || !valid(&claim.artifact_sha256)
                || !valid(&claim.result_artifact_sha256)
                || !valid(&claim.mediator_witness_sha256)
                || !intents.insert(claim.intent_id.as_str())
                || !artifacts.insert(claim.artifact_sha256.as_str())
        })
    {
        return Err(error("routine-production-reuse-claim-invalid"));
    }
    Ok(())
}

pub(crate) fn validate_reuse_preauthorization(
    payload: &Payload,
    spec: &ReservationSpec,
    authorization: &ReusePreauthorization,
    authority_id: &str,
) -> Result<(), RoutineError> {
    let Some(record) = payload.protocols.get(&spec.binding.protocol_id) else {
        return Err(error("routine-production-reuse-preauthorization-stale"));
    };
    if authorization.authority_id != authority_id
        || authorization.binding != spec.binding
        || authorization.generation != payload.generation
        || authorization.record_sha256 != sha256(&canonical(record)?)
        || record.binding != spec.binding
        || record.state != AttemptState::Complete
        || record.artifacts.is_empty()
        || record.artifacts.len() != authorization.claims.len()
        || validate_reuse_claims(&spec.binding, &authorization.claims).is_err()
        || authorization.claims.iter().any(|claim| {
            record.artifacts.get(&claim.artifact_sha256) != Some(&claim.mediator_witness_sha256)
        })
    {
        return Err(error("routine-production-reuse-preauthorization-stale"));
    }
    Ok(())
}

pub(crate) fn validate_binding(binding: &AuthorityBinding) -> Result<(), RoutineError> {
    if [
        &binding.protocol_id,
        &binding.effect_id,
        &binding.context_id,
        &binding.candidate_id,
        &binding.plan_id,
        &binding.snapshot_id,
    ]
    .into_iter()
    .all(|value| valid(value))
    {
        Ok(())
    } else {
        Err(error("routine-production-binding-invalid"))
    }
}
