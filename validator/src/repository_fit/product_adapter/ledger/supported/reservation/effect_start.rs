use super::*;

impl FileLedger {
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
            let event = next_event(NextLedgerEvent::transition(
                payload,
                current,
                RepositoryFitLedgerState::EffectStarted,
                None,
                None,
                tick,
            ))?;
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
