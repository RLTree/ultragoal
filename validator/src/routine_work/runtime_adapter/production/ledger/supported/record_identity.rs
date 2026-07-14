use super::*;

pub(crate) fn exact_record<'a>(
    payload: &'a Payload,
    token: &ReservationToken,
) -> Result<&'a ProtocolRecord, RoutineError> {
    let record = payload
        .protocols
        .get(&token.binding.protocol_id)
        .ok_or_else(|| error("routine-production-reservation-missing"))?;
    if record.binding != token.binding
        || record.request_id != token.request_id
        || record.grant_id != token.grant_id
        || record.recovery_marker != token.recovery_marker
        || record.recovery_for != token.recovery_for
        || record.reuse_only != token.reuse_only
        || record.expires_tick != token.expires_tick
    {
        return Err(error("routine-production-reservation-binding-invalid"));
    }
    Ok(record)
}

pub(crate) fn exact_record_mut<'a>(
    payload: &'a mut Payload,
    token: &ReservationToken,
) -> Result<&'a mut ProtocolRecord, RoutineError> {
    let record = payload
        .protocols
        .get_mut(&token.binding.protocol_id)
        .ok_or_else(|| error("routine-production-reservation-missing"))?;
    if record.binding != token.binding
        || record.request_id != token.request_id
        || record.grant_id != token.grant_id
        || record.recovery_marker != token.recovery_marker
        || record.recovery_for != token.recovery_for
        || record.reuse_only != token.reuse_only
        || record.expires_tick != token.expires_tick
    {
        return Err(error("routine-production-reservation-binding-invalid"));
    }
    Ok(record)
}

pub(crate) fn exact_completed_reuse_record<'a>(
    payload: &'a Payload,
    token: &ReservationToken,
) -> Result<&'a ProtocolRecord, RoutineError> {
    let record = payload
        .protocols
        .get(&token.binding.protocol_id)
        .ok_or_else(|| error("routine-production-reservation-missing"))?;
    if !token.reuse_only
        || token.recovery_for.is_some()
        || !payload.consumed_grants.contains(&token.grant_id)
        || record.binding != token.binding
        || record.state != AttemptState::Complete
        || record.artifacts.is_empty()
    {
        return Err(error("routine-production-reuse-reservation-invalid"));
    }
    Ok(record)
}

pub(crate) fn validate_reservation_capacity(
    payload: &Payload,
    spec: &ReservationSpec,
) -> Result<(), RoutineError> {
    let consumed = payload
        .consumed_grants
        .len()
        .checked_add(1)
        .ok_or_else(|| error("routine-production-authority-capacity-exhausted"))?;
    let adds_protocol =
        !spec.reuse_only && !payload.protocols.contains_key(&spec.binding.protocol_id);
    let adds_effect = !spec.reuse_only && !payload.effects.contains_key(&spec.binding.effect_id);
    let protocols = payload
        .protocols
        .len()
        .checked_add(usize::from(adds_protocol))
        .ok_or_else(|| error("routine-production-authority-capacity-exhausted"))?;
    let effects = payload
        .effects
        .len()
        .checked_add(usize::from(adds_effect))
        .ok_or_else(|| error("routine-production-authority-capacity-exhausted"))?;
    if consumed > MAX_RECORDS.saturating_mul(4) || protocols > MAX_RECORDS || effects > MAX_RECORDS
    {
        return Err(error("routine-production-authority-capacity-exhausted"));
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn test_digest(label: &str, ordinal: u64) -> String {
    sha256(format!("routine-production-test-{label}-{ordinal}").as_bytes())
}
