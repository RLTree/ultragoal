use super::*;

impl FileLedger {
    pub(crate) fn terminal(
        &self,
        token: ReservationToken,
        state: RepositoryFitLedgerState,
        terminal_sha256: &str,
        error_id: Option<AdapterErrorId>,
        tick: u64,
    ) -> Result<(), LedgerError> {
        if state != RepositoryFitLedgerState::Rejected || !valid_digest(terminal_sha256) {
            return Err(invalid_transition());
        }
        self.with_snapshot(|payload, replayed| {
            let current = replayed
                .records
                .get(&token.reservation_id)
                .ok_or_else(invalid_transition)?;
            if current.state != RepositoryFitLedgerState::Reserved
                || !token_matches(&token, current)
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
                state,
                Some(terminal_sha256),
                error_id,
                tick,
            )?;
            append(payload, event)?;
            Ok(((), true))
        })
    }
}
