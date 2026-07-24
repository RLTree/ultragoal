use super::*;

pub(super) fn exact_record<'a>(
    payload: &'a Payload,
    token: &ReservationToken,
) -> Result<&'a ProtocolRecord, RoutineError> {
    let record = payload
        .attempts
        .get(&token.grant_id)
        .ok_or_else(|| error("routine-production-reservation-missing"))?;
    exact_binding(record, token)?;
    Ok(record)
}

pub(super) fn exact_record_mut<'a>(
    payload: &'a mut Payload,
    token: &ReservationToken,
) -> Result<&'a mut ProtocolRecord, RoutineError> {
    let record = payload
        .attempts
        .get_mut(&token.grant_id)
        .ok_or_else(|| error("routine-production-reservation-missing"))?;
    exact_binding(record, token)?;
    Ok(record)
}

fn exact_binding(record: &ProtocolRecord, token: &ReservationToken) -> Result<(), RoutineError> {
    if record.binding != token.binding
        || record.request_id != token.request_id
        || record.grant_id != token.grant_id
        || record.recovery_marker != token.recovery_marker
        || record.predecessor_continuations != token.predecessor_continuations
        || record.owner != token.owner
        || record.expires_tick != token.expires_tick.get()
        || record.intents != token.intents
    {
        return Err(error("routine-production-reservation-binding-invalid"));
    }
    Ok(())
}

pub(super) fn validate_reservation_capacity(
    payload: &Payload,
    token: &ReservationToken,
) -> Result<(), RoutineError> {
    let consumed = payload
        .consumed_grants
        .len()
        .checked_add(1)
        .ok_or_else(|| error("routine-production-authority-capacity-exhausted"))?;
    let attempts = payload
        .attempts
        .len()
        .checked_add(1)
        .ok_or_else(|| error("routine-production-authority-capacity-exhausted"))?;
    let effects = payload
        .effects
        .len()
        .checked_add(usize::from(
            !payload.effects.contains_key(&token.binding.effect_id),
        ))
        .ok_or_else(|| error("routine-production-authority-capacity-exhausted"))?;
    if consumed > MAX_RECORDS.saturating_mul(4)
        || attempts > MAX_RECORDS
        || effects > MAX_RECORDS
        || payload
            .attempts
            .values()
            .any(|record| record.state.pending() && record.failure_evidence.len() >= 32)
    {
        return Err(error("routine-production-authority-capacity-exhausted"));
    }
    Ok(())
}
