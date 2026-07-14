struct ReservedPublicationAcknowledgement<'a> {
    effect: &'a AuthorizedHostEffect,
    effect_identity_sha256: &'a str,
    command_output_sha256: Vec<String>,
    outcome_sha256: String,
    completed_at_unix_ms: u64,
    committed: CommittedPublication,
}

impl<'a> SupportedHostEffectExecutor<'a> {
    fn acknowledge_reserved_publication(
        &mut self,
        request: ReservedPublicationAcknowledgement<'_>,
    ) -> Result<HostEffectExecutionReceipt, HostEffectExecutorFailure> {
        let ReservedPublicationAcknowledgement {
            effect,
            effect_identity_sha256,
            command_output_sha256,
            outcome_sha256,
            completed_at_unix_ms,
            committed,
        } = request;
        let terminal = self
            .transition_terminal(effect, HostEffectState::Settled, outcome_sha256.clone())
            .map_err(|failure| {
                self.committed_before_terminal_recovery_failure(
                    effect,
                    effect_identity_sha256,
                    &committed,
                    vec![failure.id()],
                )
            })?;
        let recovery = |error_id| {
            self.committed_recovery_failure(
                effect,
                effect_identity_sha256,
                &committed,
                terminal.current_head().clone(),
                error_id,
            )
        };
        let acknowledgement = PublicationAcknowledgementIdentity::new(
            &committed.expectation,
            terminal.current_head(),
        )
        .map_err(|_| recovery(HostEffectExecutorErrorId::PartialAcknowledgement))?;
        let acknowledgement_json = serde_json::to_vec(&acknowledgement)
            .map_err(|_| recovery(HostEffectExecutorErrorId::PartialAcknowledgement))?;
        let decoded =
            PublicationAcknowledgementIdentity::from_canonical_json(&acknowledgement_json)
                .map_err(|_| recovery(HostEffectExecutorErrorId::PartialAcknowledgement))?;
        if decoded != acknowledgement {
            return Err(recovery(HostEffectExecutorErrorId::FalsePassReceipt));
        }
        let (_, classification) = self
            .target
            .acknowledge(
                &committed,
                acknowledgement.clone(),
                terminal.current_head().clone(),
            )
            .map_err(|_| recovery(HostEffectExecutorErrorId::FalsePassReceipt))?;
        if classification.id() != PublicationClassificationId::AcknowledgedCommitted {
            return Err(recovery(HostEffectExecutorErrorId::FalsePassReceipt));
        }
        let outcome = HostEffectOutcome::new(
            effect.permit().permit_id().to_owned(),
            HostEffectState::Settled,
            command_output_sha256.clone(),
            Some(
                committed
                    .expectation
                    .publication_identity_sha256()
                    .to_owned(),
            ),
            outcome_sha256,
            completed_at_unix_ms,
        )
        .map_err(|_| recovery(HostEffectExecutorErrorId::FalsePassReceipt))?;
        Ok(HostEffectExecutionReceipt::new(
            effect_identity_sha256.to_owned(),
            outcome,
            terminal.current_head().clone(),
            command_output_sha256,
            acknowledgement,
            acknowledgement_json,
            classification,
        ))
    }
}
