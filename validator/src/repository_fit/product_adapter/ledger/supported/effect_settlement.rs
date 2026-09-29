use super::*;

impl EffectOwner<'_> {
    pub(crate) fn terminal(
        self,
        state: RepositoryFitLedgerState,
        terminal_sha256: &str,
        error_id: Option<AdapterErrorId>,
        tick: u64,
    ) -> Result<(), LedgerError> {
        if !state.terminal() || !valid_digest(terminal_sha256) {
            return Err(invalid_transition());
        }
        self.ledger
            .with_held_snapshot(&self.guard, |payload, replayed| {
                let current = replayed
                    .records
                    .get(&self.token.reservation_id)
                    .ok_or_else(invalid_transition)?;
                if current.state != RepositoryFitLedgerState::EffectStarted
                    || !token_matches(&self.token, current)
                    || tick < current.transition_tick
                {
                    return Err(invalid_transition());
                }
                let event = next_event(NextLedgerEvent::transition(
                    payload,
                    current,
                    state,
                    Some(terminal_sha256),
                    error_id,
                    tick,
                ))?;
                append(payload, event)?;
                Ok(((), true))
            })
    }
}
