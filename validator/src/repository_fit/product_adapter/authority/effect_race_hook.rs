use super::*;

#[cfg(test)]
pub(crate) fn test_after_effect_before_terminal() {
    AFTER_EFFECT_BEFORE_TERMINAL.with(|slot| {
        if let Some(action) = slot.borrow_mut().take() {
            action();
        }
    });
}

#[cfg(not(test))]
pub(crate) const fn test_after_effect_before_terminal() {}

#[cfg(test)]
pub(crate) fn test_configure_effects(effects: &mut LocalEffects) {
    CONFIGURE_EFFECTS.with(|slot| {
        if let Some(action) = slot.borrow_mut().take() {
            action(effects);
        }
    });
}

#[cfg(not(test))]
pub(crate) const fn test_configure_effects(_effects: &mut LocalEffects) {}

#[cfg(test)]
pub(crate) fn after_reservation_for_test(action: impl FnOnce() + 'static) {
    AFTER_RESERVATION.with(|slot| {
        let prior = slot.borrow_mut().replace(Box::new(action));
        assert!(
            prior.is_none(),
            "an after-reservation action is already armed"
        );
    });
}

#[cfg(test)]
pub(crate) fn after_effect_start_before_apply_for_test(action: impl FnOnce() + 'static) {
    AFTER_EFFECT_START_BEFORE_APPLY.with(|slot| {
        let prior = slot.borrow_mut().replace(Box::new(action));
        assert!(
            prior.is_none(),
            "an after-effect-start action is already armed"
        );
    });
}

#[cfg(test)]
pub(crate) fn after_effect_before_terminal_for_test(action: impl FnOnce() + 'static) {
    AFTER_EFFECT_BEFORE_TERMINAL.with(|slot| {
        let prior = slot.borrow_mut().replace(Box::new(action));
        assert!(prior.is_none(), "an after-effect action is already armed");
    });
}

#[cfg(test)]
pub(crate) fn configure_effects_for_test(action: impl FnOnce(&mut LocalEffects) + 'static) {
    CONFIGURE_EFFECTS.with(|slot| {
        let prior = slot.borrow_mut().replace(Box::new(action));
        assert!(prior.is_none(), "an effect configuration is already armed");
    });
}

#[cfg(target_vendor = "apple")]
pub(crate) fn settle_pre_effect_failure(
    ledger: &FileRepositoryFitLedger,
    token: ReservationToken,
    request_id: String,
    error: FitAdapterError,
    tick: u64,
) -> RepositoryFitProductionOutcome {
    let terminal = terminal_digest(
        &request_id,
        RepositoryFitLedgerState::Rejected,
        Some(error.id()),
        false,
        false,
        None,
    );
    match ledger.terminal(
        token,
        RepositoryFitLedgerState::Rejected,
        &terminal,
        Some(error.id()),
        tick,
    ) {
        Ok(()) => RepositoryFitProductionOutcome::terminal_failure(
            request_id,
            error,
            RepositoryFitLedgerState::Rejected,
            false,
            false,
        ),
        Err(ledger_error) => RepositoryFitProductionOutcome::terminal_failure(
            request_id,
            ledger_error.adapter_error(),
            RepositoryFitLedgerState::Reserved,
            false,
            false,
        ),
    }
}

#[cfg(target_vendor = "apple")]
pub(crate) fn settle_success(
    owner: EffectOwner<'_>,
    request_id: String,
    outcome: RepositoryFitApplyOutcome,
    tick: u64,
) -> RepositoryFitProductionOutcome {
    let outcome_sha256 = match serde_json::to_vec(&outcome) {
        Ok(bytes) => digest(&bytes),
        Err(_) => {
            return RepositoryFitProductionOutcome::terminal_failure(
                request_id,
                adapter_error(AdapterErrorId::ApplyOutcomeAmbiguous),
                RepositoryFitLedgerState::EffectStarted,
                true,
                false,
            );
        }
    };
    match owner.terminal(
        RepositoryFitLedgerState::Committed,
        &outcome_sha256,
        None,
        tick,
    ) {
        Ok(()) => RepositoryFitProductionOutcome::success(request_id, outcome),
        Err(_) => RepositoryFitProductionOutcome::terminal_failure(
            request_id,
            adapter_error(AdapterErrorId::ApplyOutcomeAmbiguous),
            RepositoryFitLedgerState::EffectStarted,
            true,
            false,
        ),
    }
}

#[cfg(target_vendor = "apple")]
pub(crate) fn settle_failure<E: super::super::root_permit::RepositoryFitPermitEffects>(
    owner: EffectOwner<'_>,
    request_id: String,
    failure: RepositoryFitApplyFailure<E>,
    tick: u64,
) -> RepositoryFitProductionOutcome {
    let error = failure.error();
    let effect_started = failure.effect_started();
    let rollback_complete = failure.rollback_complete();
    let state = if !effect_started {
        RepositoryFitLedgerState::Rejected
    } else if rollback_complete {
        RepositoryFitLedgerState::RolledBack
    } else {
        RepositoryFitLedgerState::Ambiguous
    };
    let terminal = terminal_digest(
        &request_id,
        state,
        Some(error.id()),
        effect_started,
        rollback_complete,
        None,
    );
    match owner.terminal(state, &terminal, Some(error.id()), tick) {
        Ok(()) => RepositoryFitProductionOutcome::terminal_failure(
            request_id,
            error,
            state,
            effect_started,
            rollback_complete,
        ),
        Err(_) => RepositoryFitProductionOutcome::terminal_failure(
            request_id,
            adapter_error(if effect_started {
                AdapterErrorId::ApplyOutcomeAmbiguous
            } else {
                AdapterErrorId::ApplyOutcomeInvalid
            }),
            RepositoryFitLedgerState::EffectStarted,
            effect_started,
            false,
        ),
    }
}
