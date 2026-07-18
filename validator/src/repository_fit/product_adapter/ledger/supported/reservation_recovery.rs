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
            let event = next_event(
                payload,
                &current.reservation_id,
                &current.binding_sha256,
                &current.semantic_effect_id,
                &current.target_scope_id,
                &current.permit_id,
                &current.nonce_sha256,
                &current.recovery_intent_sha256,
                current.issued_tick,
                current.expires_tick,
                &current.recovery,
                terminal.state,
                Some(&terminal.terminal_sha256),
                terminal.error_id,
                tick,
            )?;
            append(payload, event)?;
            Ok((terminal.state, true))
        })
    }
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
    pub(crate) fn with_snapshot<T>(
        &self,
        operation: impl FnOnce(&mut SnapshotPayload, &ReplayState) -> Result<(T, bool), LedgerError>,
    ) -> Result<T, LedgerError> {
        let guard = self.acquire_process_lock()?;
        self.with_held_snapshot(&guard, operation)
    }
    pub(crate) fn acquire_process_lock(&self) -> Result<ProcessLock, LedgerError> {
        self.store.verify_root()?;
        let lock = self.store.open_existing(LOCK_NAME, libc::O_RDWR)?;
        if exact_identity(&self.store, LOCK_NAME, &lock, 0o600)? != self.lock_identity {
            return Err(tampered());
        }
        test_before_lock_acquire();
        let guard = ProcessLock::acquire(lock)?;
        if exact_identity(&self.store, LOCK_NAME, &guard.0, 0o600)? != self.lock_identity {
            return Err(tampered());
        }
        self.verify_store()?;
        Ok(guard)
    }
}
