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
            let event = next_event(NextLedgerEvent::reserved(
                payload,
                &reservation_id,
                &request,
            ))?;
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
}
