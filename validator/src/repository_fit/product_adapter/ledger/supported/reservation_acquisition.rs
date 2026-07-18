use super::*;

impl FileLedger {
    pub(crate) fn reserve(
        &self,
        request: ReservationRequest<'_>,
    ) -> Result<ReservationDecision, LedgerError> {
        validate_reservation(&request)?;
        self.with_snapshot(|payload, replayed| {
            if let Some(owner) = replayed.nonce_owner.get(request.nonce_sha256) {
                let current = replayed.records.get(owner).ok_or_else(tampered)?;
                if current.semantic_effect_id != request.semantic_effect_id
                    || current.recovery_intent_sha256 != request.recovery_intent_sha256
                    || current.recovery != *request.recovery
                {
                    return Err(replay_error());
                }
                return Ok((ReservationDecision::Existing(existing(current)), false));
            }
            if let Some(owner) = replayed.semantic_owner.get(request.semantic_effect_id) {
                let current = replayed.records.get(owner).ok_or_else(tampered)?;
                return Ok((ReservationDecision::Existing(existing(current)), false));
            }
            if replayed
                .active_targets
                .contains_key(request.target_scope_id)
            {
                return Err(active_lease());
            }
            let reservation_id = digest(
                &serde_json::to_vec(&(
                    "repository-fit-ledger-reservation-v3",
                    &self.authority_id,
                    request.binding_sha256,
                    request.semantic_effect_id,
                    request.target_scope_id,
                    request.permit_id,
                    request.nonce_sha256,
                    request.recovery_intent_sha256,
                ))
                .map_err(|_| invalid_transition())?,
            );
            let event = next_event(
                payload,
                &reservation_id,
                request.binding_sha256,
                request.semantic_effect_id,
                request.target_scope_id,
                request.permit_id,
                request.nonce_sha256,
                request.recovery_intent_sha256,
                request.issued_tick,
                request.expires_tick,
                request.recovery,
                RepositoryFitLedgerState::Reserved,
                None,
                None,
                request.issued_tick,
            )?;
            append(payload, event)?;
            Ok((
                ReservationDecision::Acquired(ReservationToken {
                    reservation_id,
                    binding_sha256: request.binding_sha256.to_owned(),
                    semantic_effect_id: request.semantic_effect_id.to_owned(),
                    target_scope_id: request.target_scope_id.to_owned(),
                    permit_id: request.permit_id.to_owned(),
                    nonce_sha256: request.nonce_sha256.to_owned(),
                    recovery_intent_sha256: request.recovery_intent_sha256.to_owned(),
                }),
                true,
            ))
        })
    }
    pub(crate) fn lookup_by_nonce(
        &self,
        nonce_sha256: &str,
    ) -> Result<Option<ExistingReservation>, LedgerError> {
        if !valid_digest(nonce_sha256) {
            return Err(invalid_transition());
        }
        self.with_snapshot(|_, replayed| {
            let existing = replayed
                .nonce_owner
                .get(nonce_sha256)
                .map(|owner| {
                    replayed
                        .records
                        .get(owner)
                        .map(existing)
                        .ok_or_else(tampered)
                })
                .transpose()?;
            Ok((existing, false))
        })
    }
    pub(crate) fn begin_effect(
        &self,
        token: ReservationToken,
        tick: u64,
    ) -> Result<EffectOwner<'_>, LedgerError> {
        let guard = self.acquire_process_lock()?;
        self.with_held_snapshot(&guard, |payload, replayed| {
            let current = replayed
                .records
                .get(&token.reservation_id)
                .ok_or_else(invalid_transition)?;
            if !token_matches(&token, current)
                || current.state != RepositoryFitLedgerState::Reserved
                || tick < current.transition_tick
                || tick > current.expires_tick
            {
                return Err(invalid_transition());
            }
            let event = next_event(
                payload,
                &token.reservation_id,
                &token.binding_sha256,
                &token.semantic_effect_id,
                &token.target_scope_id,
                &token.permit_id,
                &token.nonce_sha256,
                &token.recovery_intent_sha256,
                current.issued_tick,
                current.expires_tick,
                &current.recovery,
                RepositoryFitLedgerState::EffectStarted,
                None,
                None,
                tick,
            )?;
            append(payload, event)?;
            Ok(((), true))
        })?;
        Ok(EffectOwner {
            ledger: self,
            guard,
            token,
        })
    }
}
