use super::*;

pub(crate) fn same_reservation(left: &LedgerEvent, right: &LedgerEvent) -> bool {
    left.reservation_id == right.reservation_id
        && left.binding_sha256 == right.binding_sha256
        && left.semantic_effect_id == right.semantic_effect_id
        && left.target_scope_id == right.target_scope_id
        && left.permit_id == right.permit_id
        && left.nonce_sha256 == right.nonce_sha256
        && left.recovery_intent_sha256 == right.recovery_intent_sha256
        && left.issued_tick == right.issued_tick
        && left.expires_tick == right.expires_tick
        && left.recovery == right.recovery
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn next_event(
    payload: &SnapshotPayload,
    reservation_id: &str,
    binding_sha256: &str,
    semantic_effect_id: &str,
    target_scope_id: &str,
    permit_id: &str,
    nonce_sha256: &str,
    recovery_intent_sha256: &str,
    issued_tick: u64,
    expires_tick: u64,
    recovery: &RecoveryTargetSpec,
    state: RepositoryFitLedgerState,
    terminal_sha256: Option<&str>,
    error_id: Option<AdapterErrorId>,
    transition_tick: u64,
) -> Result<LedgerEvent, LedgerError> {
    let sequence = payload.generation.checked_add(1).ok_or_else(tampered)?;
    let event_id = digest(
        &serde_json::to_vec(&(
            "repository-fit-ledger-event-id-v2",
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
        recovery_intent_sha256: recovery_intent_sha256.to_owned(),
        issued_tick,
        expires_tick,
        recovery: recovery.clone(),
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

pub(crate) fn event_digest(event: &LedgerEvent) -> Result<String, LedgerError> {
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
        &event.recovery_intent_sha256,
        event.issued_tick,
        event.expires_tick,
        (
            &event.recovery,
            event.state,
            &event.terminal_sha256,
            event.error_id,
            event.transition_tick,
        ),
    ))
    .map(|bytes| digest(&bytes))
    .map_err(|_| invalid_transition())
}

pub(crate) fn append(payload: &mut SnapshotPayload, event: LedgerEvent) -> Result<(), LedgerError> {
    if payload.events.len() >= MAX_EVENTS || event.sequence != payload.generation + 1 {
        return Err(invalid_transition());
    }
    payload.generation = event.sequence;
    payload.head_sha256.clone_from(&event.event_sha256);
    payload.events.push(event);
    Ok(())
}

pub(crate) fn existing(event: &LedgerEvent) -> ExistingReservation {
    ExistingReservation {
        reservation_id: event.reservation_id.clone(),
        state: event.state,
        expires_tick: event.expires_tick,
        recovery_intent_sha256: event.recovery_intent_sha256.clone(),
        recovery: event.recovery.clone(),
    }
}

pub(crate) fn validate_reservation(request: &ReservationRequest<'_>) -> Result<(), LedgerError> {
    if [
        request.binding_sha256,
        request.semantic_effect_id,
        request.target_scope_id,
        request.permit_id,
        request.nonce_sha256,
        request.recovery_intent_sha256,
    ]
    .iter()
    .any(|value| !valid_digest(value))
        || request.expires_tick < request.issued_tick
        || !valid_recovery(&request.recovery)
        || !recovery_intent_matches(request.recovery, request.recovery_intent_sha256)
    {
        return Err(invalid_transition());
    }
    Ok(())
}

pub(crate) fn valid_recovery(recovery: &RecoveryTargetSpec) -> bool {
    let leaf_paths = recovery
        .rows
        .iter()
        .map(|row| row.path.clone())
        .collect::<Vec<_>>();
    valid_digest(&recovery.request_id)
        && valid_digest(&recovery.root_binding)
        && recovery.rows.len() <= MAX_RECOVERY_ROWS
        && recovery.ancestors.valid_for_leaf_paths(&leaf_paths)
        && recovery.rows.iter().all(|row| {
            CanonicalPath::parse(&row.path).is_ok()
                && row.pre_sha256.as_deref().is_none_or(valid_digest)
                && row
                    .pre_mode
                    .is_none_or(|mode| matches!(mode, 0o644 | 0o755))
                && row.pre_sha256.is_some() == row.pre_mode.is_some()
                && valid_digest(&row.post_sha256)
                && matches!(row.post_mode, 0o644 | 0o755)
        })
        && recovery
            .rows
            .windows(2)
            .all(|rows| rows[0].path.as_bytes() < rows[1].path.as_bytes())
}

pub(crate) fn recovery_intent_matches(recovery: &RecoveryTargetSpec, expected: &str) -> bool {
    canonical_recovery_intent_bytes(recovery).is_some_and(|bytes| digest(&bytes) == expected)
}

pub(crate) fn token_matches(token: &ReservationToken, event: &LedgerEvent) -> bool {
    event.reservation_id == token.reservation_id
        && event.binding_sha256 == token.binding_sha256
        && event.semantic_effect_id == token.semantic_effect_id
        && event.target_scope_id == token.target_scope_id
        && event.permit_id == token.permit_id
        && event.nonce_sha256 == token.nonce_sha256
        && event.recovery_intent_sha256 == token.recovery_intent_sha256
}
