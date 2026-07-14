impl<'a> SupportedHostEffectExecutor<'a> {
    fn finish_publication_failure(
        &self,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        command_digests: Vec<CommandCaptureDigest>,
        completed_at_unix_ms: u64,
        failure: PublicationFailure,
    ) -> Result<HostEffectExecutionReceipt, HostEffectExecutorFailure> {
        let mut originating_error_ids = vec![failure.id];
        let ambiguous_outcome_sha256 = match outcome_digest(
            effect_identity_sha256,
            effect,
            HostEffectState::Ambiguous,
            &command_digests,
            self.policy.policy_sha256(),
            completed_at_unix_ms,
        ) {
            Ok(outcome_sha256) => outcome_sha256,
            Err(outcome_failure) => {
                append_error_cause(&mut originating_error_ids, outcome_failure.id());
                return Err(self.post_reservation_recovery_failure(
                    effect,
                    effect_identity_sha256,
                    HostEffectExecutorErrorId::RecoveryRequired,
                    originating_error_ids,
                    Some(PriorPublicationEvidence {
                        publication_identity_sha256: None,
                        observation: failure.observation.as_ref(),
                    }),
                ));
            }
        };
        let terminal = match self.transition_terminal(
            effect,
            HostEffectState::Ambiguous,
            ambiguous_outcome_sha256,
        ) {
            Ok(terminal) => terminal,
            Err(transition_failure) => {
                append_error_cause(&mut originating_error_ids, transition_failure.id());
                return Err(self.publication_before_terminal_recovery_failure(
                    effect,
                    effect_identity_sha256,
                    &failure,
                    originating_error_ids,
                ));
            }
        };
        let (observation, classification) = match self
            .target
            .reobserve_failure(&failure, terminal.current_head().clone())
        {
            Ok(current) => current,
            Err(observation_failure) => {
                append_error_cause(&mut originating_error_ids, observation_failure.id());
                return Err(self.post_reservation_recovery_failure(
                    effect,
                    effect_identity_sha256,
                    HostEffectExecutorErrorId::RecoveryRequired,
                    originating_error_ids,
                    Some(PriorPublicationEvidence {
                        publication_identity_sha256: None,
                        observation: failure.observation.as_ref(),
                    }),
                ));
            }
        };
        let recovery = HostEffectRecoveryHandoff::publication(
            effect_identity_sha256.to_owned(),
            effect.permit().permit_id().to_owned(),
            terminal.current_head().clone(),
            observation,
            classification,
            originating_error_ids,
        );
        debug_assert!(recovery.verify_binding());
        Err(HostEffectExecutorFailure::with_recovery(
            failure.id,
            Some(HostEffectState::Ambiguous),
            recovery,
        ))
    }

    fn committed_recovery_failure(
        &self,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        committed: &CommittedPublication,
        ledger_head: HostEffectLedgerHead,
        id: HostEffectExecutorErrorId,
    ) -> HostEffectExecutorFailure {
        match self
            .target
            .reobserve_committed(committed, ledger_head.clone())
        {
            Ok((observation, classification)) => {
                let recovery = HostEffectRecoveryHandoff::publication(
                    effect_identity_sha256.to_owned(),
                    effect.permit().permit_id().to_owned(),
                    ledger_head,
                    observation,
                    classification,
                    vec![id],
                );
                debug_assert!(recovery.verify_binding());
                HostEffectExecutorFailure::with_recovery(
                    id,
                    Some(HostEffectState::Settled),
                    recovery,
                )
            }
            Err(observation_failure) => self.post_reservation_recovery_failure(
                effect,
                effect_identity_sha256,
                id,
                vec![id, observation_failure.id()],
                Some(PriorPublicationEvidence {
                    publication_identity_sha256: Some(
                        committed.expectation.publication_identity_sha256(),
                    ),
                    observation: Some(&committed.observation),
                }),
            ),
        }
    }

    fn committed_before_terminal_recovery_failure(
        &self,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        committed: &CommittedPublication,
        originating_error_ids: Vec<HostEffectExecutorErrorId>,
    ) -> HostEffectExecutorFailure {
        let mut originating_error_ids = originating_error_ids;
        let ledger_head = match self.ledger.head() {
            Ok(ledger_head) => ledger_head,
            Err(_ledger_observation_failure) => {
                append_error_cause(
                    &mut originating_error_ids,
                    HostEffectExecutorErrorId::LedgerSubstitution,
                );
                effect.record().current_head().clone()
            }
        };
        match self
            .target
            .reobserve_committed(committed, ledger_head.clone())
        {
            Ok((observation, classification)) => {
                let recovery = HostEffectRecoveryHandoff::publication(
                    effect_identity_sha256.to_owned(),
                    effect.permit().permit_id().to_owned(),
                    ledger_head,
                    observation,
                    classification,
                    originating_error_ids,
                );
                debug_assert!(recovery.verify_binding());
                HostEffectExecutorFailure::with_recovery(
                    HostEffectExecutorErrorId::RecoveryRequired,
                    None,
                    recovery,
                )
            }
            Err(observation_failure) => {
                append_error_cause(&mut originating_error_ids, observation_failure.id());
                let recovery =
                    HostEffectRecoveryHandoff::post_publication_terminal_transition_observation_unavailable(
                        effect_identity_sha256.to_owned(),
                        effect.permit().permit_id().to_owned(),
                        ledger_head,
                        committed
                            .expectation
                            .publication_identity_sha256()
                            .to_owned(),
                        committed.observation.clone(),
                        originating_error_ids,
                    );
                debug_assert!(recovery.verify_binding());
                HostEffectExecutorFailure::with_recovery(
                    HostEffectExecutorErrorId::RecoveryRequired,
                    None,
                    recovery,
                )
            }
        }
    }
}
