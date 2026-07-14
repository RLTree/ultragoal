use super::*;

#[cfg(target_vendor = "apple")]
pub(crate) fn recover_existing(
    context: &LiveContext,
    ledger: &FileRepositoryFitLedger,
    existing: super::super::ledger::ExistingReservation,
    intent: &RepositoryFitRecoveryIntent,
    recovery_tick: u64,
    request_id: String,
) -> RepositoryFitProductionOutcome {
    let observed_state = existing.state();
    let observed_effect_started = observed_state == RepositoryFitLedgerState::EffectStarted;
    if !existing.matches_intent(intent.sha256(), intent.recovery()) || observed_state.terminal() {
        return RepositoryFitProductionOutcome::causal_refusal(
            request_id,
            adapter_error(AdapterErrorId::ApplyPermitReplayed),
            observed_state,
            observed_effect_started,
        );
    }
    if recovery_tick <= existing.expires_tick() {
        return RepositoryFitProductionOutcome::causal_refusal(
            request_id,
            adapter_error(AdapterErrorId::ApplyLeaseInvalid),
            observed_state,
            observed_effect_started,
        );
    }
    let original_request_id = existing.request_id().to_owned();
    let prepared_root_binding = intent.recovery().root_binding.clone();
    let recovered_phase = std::cell::Cell::new(observed_state);
    match ledger.reconcile_expired(
        existing,
        intent.sha256(),
        intent.recovery(),
        recovery_tick,
        |phase, recovery| {
            recovered_phase.set(phase);
            let (state, error_id) = match phase {
                RepositoryFitLedgerState::Reserved => (
                    RepositoryFitLedgerState::Interrupted,
                    Some(AdapterErrorId::ApplyOutcomeAmbiguous),
                ),
                RepositoryFitLedgerState::EffectStarted => {
                    match classify_recovery_target(context, &prepared_root_binding, recovery) {
                        RecoveryClassification::Preimage => (
                            RepositoryFitLedgerState::Interrupted,
                            Some(AdapterErrorId::ApplyOutcomeAmbiguous),
                        ),
                        RecoveryClassification::Postimage => {
                            (RepositoryFitLedgerState::Committed, None)
                        }
                        RecoveryClassification::Other => (
                            RepositoryFitLedgerState::Ambiguous,
                            Some(AdapterErrorId::ApplyOutcomeAmbiguous),
                        ),
                    }
                }
                _ => (
                    RepositoryFitLedgerState::Ambiguous,
                    Some(AdapterErrorId::ApplyOutcomeInvalid),
                ),
            };
            let effect_started = phase == RepositoryFitLedgerState::EffectStarted;
            RecoveryTerminal {
                state,
                terminal_sha256: terminal_digest(
                    &recovery.request_id,
                    state,
                    error_id,
                    effect_started,
                    false,
                    None,
                ),
                error_id,
            }
        },
    ) {
        Ok(state) => RepositoryFitProductionOutcome::recovered(
            original_request_id,
            state,
            recovered_phase.get() == RepositoryFitLedgerState::EffectStarted,
        ),
        Err(error) => {
            let id = match error.id() {
                LedgerErrorId::Replay => AdapterErrorId::ApplyPermitReplayed,
                LedgerErrorId::ActiveLease => AdapterErrorId::ApplyLeaseInvalid,
                _ => error.adapter_error().id(),
            };
            RepositoryFitProductionOutcome::causal_refusal(
                request_id,
                adapter_error(id),
                observed_state,
                observed_effect_started,
            )
        }
    }
}

#[cfg(target_vendor = "apple")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RecoveryClassification {
    Preimage,
    Postimage,
    Other,
}

#[cfg(target_vendor = "apple")]
pub(crate) fn classify_recovery_target(
    context: &LiveContext,
    prepared_root_binding: &str,
    recovery: &RecoveryTargetSpec,
) -> RecoveryClassification {
    if context.revalidate().is_err() {
        return RecoveryClassification::Other;
    }
    let paths = match recovery
        .rows
        .iter()
        .map(|row| CanonicalPath::parse(&row.path))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(paths) => paths,
        Err(_) => return RecoveryClassification::Other,
    };
    let observation = match observe_recovery_target_contract(
        context.worktree_root(),
        &paths,
        &recovery.ancestors,
    ) {
        Ok(observation) => observation,
        Err(_) => return RecoveryClassification::Other,
    };
    if observation.root_binding != prepared_root_binding
        || observation.root_binding != recovery.root_binding
    {
        return RecoveryClassification::Other;
    }
    if context.revalidate().is_err() {
        return RecoveryClassification::Other;
    }
    let mut preimage = observation.ancestor_preimage;
    let mut postimage = observation.ancestor_postimage;
    for (observed, expected) in observation.leaves.iter().zip(&recovery.rows) {
        if observed.path != expected.path {
            return RecoveryClassification::Other;
        }
        preimage &= match (&expected.pre_sha256, expected.pre_mode) {
            (None, None) => observed.payload_sha256.is_none() && observed.mode.is_none(),
            (Some(sha256), Some(mode)) => {
                observed.valid_managed_leaf
                    && observed.payload_sha256.as_deref() == Some(sha256.as_str())
                    && observed.mode == Some(mode)
            }
            _ => false,
        };
        postimage &= observed.valid_managed_leaf
            && observed.payload_sha256.as_deref() == Some(expected.post_sha256.as_str())
            && observed.mode == Some(expected.post_mode);
    }
    if postimage {
        RecoveryClassification::Postimage
    } else if preimage {
        RecoveryClassification::Preimage
    } else {
        RecoveryClassification::Other
    }
}

#[cfg(test)]
pub(crate) fn test_after_reservation() {
    AFTER_RESERVATION.with(|slot| {
        if let Some(action) = slot.borrow_mut().take() {
            action();
        }
    });
}

#[cfg(not(test))]
pub(crate) const fn test_after_reservation() {}

#[cfg(test)]
pub(crate) fn test_after_effect_start_before_apply() {
    AFTER_EFFECT_START_BEFORE_APPLY.with(|slot| {
        if let Some(action) = slot.borrow_mut().take() {
            action();
        }
    });
}

#[cfg(not(test))]
pub(crate) const fn test_after_effect_start_before_apply() {}
