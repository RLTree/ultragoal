impl<'a> SupportedHostEffectExecutor<'a> {
    fn publication_before_terminal_recovery_failure(
        &self,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        failure: &PublicationFailure,
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
        match self.target.reobserve_failure(failure, ledger_head.clone()) {
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
                self.post_reservation_recovery_failure(
                    effect,
                    effect_identity_sha256,
                    HostEffectExecutorErrorId::RecoveryRequired,
                    originating_error_ids,
                    Some(PriorPublicationEvidence {
                        publication_identity_sha256: None,
                        observation: failure.observation.as_ref(),
                    }),
                )
            }
        }
    }

    fn post_reservation_recovery_failure(
        &self,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        returned_error_id: HostEffectExecutorErrorId,
        mut originating_error_ids: Vec<HostEffectExecutorErrorId>,
        publication: Option<PriorPublicationEvidence<'_>>,
    ) -> HostEffectExecutorFailure {
        let ledger = self.reobserve_post_reservation(effect);
        if let Some(observation_error_id) = ledger.observation_error_id {
            append_error_cause(&mut originating_error_ids, observation_error_id);
        }
        let (
            publication_identity_sha256,
            prior_publication_observation,
            publication_classification,
        ) = match publication {
            Some(evidence) => {
                let classification = if evidence.observation.is_some() {
                    HostEffectPostReservationPublicationClassification::PriorObservationCurrentObservationUnavailable
                } else if evidence.publication_identity_sha256.is_some() {
                    HostEffectPostReservationPublicationClassification::PublicationIdentityOnlyCurrentObservationUnavailable
                } else {
                    HostEffectPostReservationPublicationClassification::PublicationEvidenceUnavailable
                };
                (
                    evidence.publication_identity_sha256.map(str::to_owned),
                    evidence.observation.cloned(),
                    classification,
                )
            }
            None => (
                None,
                None,
                HostEffectPostReservationPublicationClassification::NoPublicationEvidence,
            ),
        };
        let recovery =
            HostEffectRecoveryHandoff::post_reservation(PostReservationRecoveryRequest {
                effect_identity_sha256: effect_identity_sha256.to_owned(),
                permit_id: effect.permit().permit_id().to_owned(),
                reservation_ledger_head: effect.record().current_head().clone(),
                reservation_ledger_record: effect.record().clone(),
                ledger_head: ledger.ledger_head,
                ledger_record: ledger.ledger_record,
                exact_current_ledger_observation: ledger.exact_current_observation,
                publication_identity_sha256,
                prior_publication_observation,
                originating_error_ids,
                ledger_classification: ledger.classification,
                publication_classification,
            });
        debug_assert!(recovery.verify_binding());
        HostEffectExecutorFailure::with_recovery(returned_error_id, ledger.terminal_state, recovery)
    }
}
