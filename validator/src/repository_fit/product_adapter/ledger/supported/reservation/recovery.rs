use super::*;

impl FileLedger {
    pub(crate) fn reconcile_expired(
        &self,
        existing: ExistingReservation,
        recovery_intent_sha256: &str,
        recovery: &RecoveryTargetSpec,
        tick: u64,
        reconcile: impl FnOnce(RepositoryFitLedgerState, &RecoveryTargetSpec) -> RecoveryTerminal,
    ) -> Result<RepositoryFitLedgerState, LedgerError> {
        let guard = self.acquire_process_lock()?;
        self.with_held_snapshot(&guard, |payload, replayed| {
            let current = replayed
                .records
                .get(&existing.reservation_id)
                .ok_or_else(invalid_transition)?;
            if current.expires_tick != existing.expires_tick
                || current.recovery_intent_sha256 != recovery_intent_sha256
                || current.recovery != *recovery
                || current.recovery != existing.recovery
            {
                return Err(invalid_transition());
            }
            if current.state.terminal() {
                return Err(replay_error());
            }
            if tick <= current.expires_tick || tick < current.transition_tick {
                return Err(active_lease());
            }
            let terminal = reconcile(current.state, &current.recovery);
            let allowed = match current.state {
                RepositoryFitLedgerState::Reserved => {
                    terminal.state == RepositoryFitLedgerState::Interrupted
                }
                RepositoryFitLedgerState::EffectStarted => matches!(
                    terminal.state,
                    RepositoryFitLedgerState::Interrupted
                        | RepositoryFitLedgerState::Committed
                        | RepositoryFitLedgerState::Ambiguous
                ),
                _ => false,
            };
            if !allowed
                || !valid_digest(&terminal.terminal_sha256)
                || (terminal.state == RepositoryFitLedgerState::Committed
                    && terminal.error_id.is_some())
                || (terminal.state != RepositoryFitLedgerState::Committed
                    && terminal.error_id.is_none())
            {
                return Err(invalid_transition());
            }
            let event = next_event(NextLedgerEvent::transition(
                payload,
                current,
                terminal.state,
                Some(&terminal.terminal_sha256),
                terminal.error_id,
                tick,
            ))?;
            append(payload, event)?;
            Ok((terminal.state, true))
        })
    }
}
