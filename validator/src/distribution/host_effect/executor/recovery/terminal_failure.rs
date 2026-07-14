impl<'a> SupportedHostEffectExecutor<'a> {
    fn verified_terminal_failure(
        &self,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        terminal: HostEffectLedgerRecord,
        outcome: HostEffectOutcome,
        returned_error_id: HostEffectExecutorErrorId,
        originating_error_ids: Vec<HostEffectExecutorErrorId>,
    ) -> HostEffectExecutorFailure {
        let terminal_state = terminal.state();
        let recovery =
            HostEffectRecoveryHandoff::terminal_transition(TerminalTransitionRecoveryRequest {
                effect_identity_sha256: effect_identity_sha256.to_owned(),
                permit_id: effect.permit().permit_id().to_owned(),
                ledger_head: terminal.current_head().clone(),
                ledger_record: terminal,
                exact_current_ledger_observation: true,
                outcome,
                originating_error_ids,
                classification:
                    HostEffectTerminalRecoveryClassification::TerminalCommittedAndVerified,
            });
        debug_assert!(recovery.verify_binding());
        HostEffectExecutorFailure::with_recovery(returned_error_id, Some(terminal_state), recovery)
    }

    fn prepublication_terminal_recovery_failure(
        &self,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        outcome: HostEffectOutcome,
        originating_error_ids: Vec<HostEffectExecutorErrorId>,
    ) -> HostEffectExecutorFailure {
        let (ledger_record, ledger_head, exact, classification, terminal_state) = self
            .reobserve_terminal_failure(effect, &outcome)
            .unwrap_or_else(|| {
                (
                    effect.record().clone(),
                    effect.record().current_head().clone(),
                    false,
                    HostEffectTerminalRecoveryClassification::LedgerObservationUnavailable,
                    None,
                )
            });
        let recovery =
            HostEffectRecoveryHandoff::terminal_transition(TerminalTransitionRecoveryRequest {
                effect_identity_sha256: effect_identity_sha256.to_owned(),
                permit_id: effect.permit().permit_id().to_owned(),
                ledger_head,
                ledger_record,
                exact_current_ledger_observation: exact,
                outcome,
                originating_error_ids,
                classification,
            });
        debug_assert!(recovery.verify_binding());
        HostEffectExecutorFailure::with_recovery(
            HostEffectExecutorErrorId::RecoveryRequired,
            terminal_state,
            recovery,
        )
    }

    fn identity_bearing_preflight_ledger_failure(
        &self,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        originating_error_id: HostEffectExecutorErrorId,
    ) -> HostEffectExecutorFailure {
        let outcome_sha256 = outcome_digest(
            effect_identity_sha256,
            effect,
            HostEffectState::Failed,
            &[],
            self.policy.policy_sha256(),
            0,
        )
        .expect("preflight recovery outcome serialization is infallible");
        let outcome = terminal_outcome(effect, HostEffectState::Failed, &[], outcome_sha256, 0)
            .expect("preflight recovery outcome construction is infallible");
        let recovery =
            HostEffectRecoveryHandoff::terminal_transition(TerminalTransitionRecoveryRequest {
                effect_identity_sha256: effect_identity_sha256.to_owned(),
                permit_id: effect.permit().permit_id().to_owned(),
                ledger_head: effect.record().current_head().clone(),
                ledger_record: effect.record().clone(),
                exact_current_ledger_observation: false,
                outcome,
                originating_error_ids: vec![originating_error_id],
                classification:
                    HostEffectTerminalRecoveryClassification::LedgerObservationUnavailable,
            });
        debug_assert!(recovery.verify_binding());
        HostEffectExecutorFailure::with_recovery(
            HostEffectExecutorErrorId::RecoveryRequired,
            None,
            recovery,
        )
    }

    fn reobserve_terminal_failure(
        &self,
        effect: &AuthorizedHostEffect,
        outcome: &HostEffectOutcome,
    ) -> Option<(
        HostEffectLedgerRecord,
        HostEffectLedgerHead,
        bool,
        HostEffectTerminalRecoveryClassification,
        Option<HostEffectState>,
    )> {
        const MAX_STABLE_OBSERVATION_ATTEMPTS: usize = 3;
        for _ in 0..MAX_STABLE_OBSERVATION_ATTEMPTS {
            let Ok(head_before) = self.ledger.head() else {
                continue;
            };
            let Ok(Some(record_before)) = self.ledger.read(effect.permit().permit_id()) else {
                continue;
            };
            let Ok(Some(record_after)) = self.ledger.read(effect.permit().permit_id()) else {
                continue;
            };
            let Ok(head_after) = self.ledger.head() else {
                continue;
            };
            if head_before != head_after || record_before != record_after {
                continue;
            }
            if head_after != *record_after.current_head()
                || record_after.reservation().permit_id() != effect.permit().permit_id()
            {
                continue;
            }
            if record_after == *effect.record() {
                return Some((
                    record_after,
                    head_after,
                    true,
                    HostEffectTerminalRecoveryClassification::StillInFlight,
                    None,
                ));
            }
            if record_after.reservation() == effect.record().reservation()
                && record_after.state() == outcome.state
                && record_after.outcome_sha256.as_deref() == Some(outcome.outcome_sha256.as_str())
            {
                return Some((
                    record_after,
                    head_after,
                    true,
                    HostEffectTerminalRecoveryClassification::TerminalCommittedButUnverifiable,
                    Some(outcome.state),
                ));
            }
            return Some((
                record_after,
                head_after,
                true,
                HostEffectTerminalRecoveryClassification::LedgerObservationRejected,
                None,
            ));
        }
        None
    }
}
