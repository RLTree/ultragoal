impl<'a> SupportedHostEffectExecutor<'a> {
    fn publish_reserved(
        &mut self,
        effect: &AuthorizedHostEffect,
        prepared: PreparedHostEffectExecution,
        command_digests: Vec<CommandCaptureDigest>,
        clock: &mut dyn RootTrustedClock,
    ) -> Result<HostEffectExecutionReceipt, HostEffectExecutorFailure> {
        let completed_at_unix_ms = match clock.sample() {
            Ok(sample) => sample.unix_ms(),
            Err(_) => {
                return self.finish_terminal_with_timestamp(
                    effect,
                    &prepared.effect_identity_sha256,
                    command_digests,
                    HostEffectExecutorErrorId::Io,
                    HostEffectState::Ambiguous,
                    0,
                );
            }
        };
        let command_output_sha256 = command_digests
            .iter()
            .map(|capture| capture.combined_sha256.clone())
            .collect::<Vec<_>>();
        let outcome_sha256 = outcome_digest(
            &prepared.effect_identity_sha256,
            effect,
            HostEffectState::Settled,
            &command_digests,
            self.policy.policy_sha256(),
            completed_at_unix_ms,
        )?;
        let publication_bytes = publication_bytes(
            &prepared.effect_identity_sha256,
            effect,
            &command_digests,
            self.policy.policy_sha256(),
            &outcome_sha256,
            completed_at_unix_ms,
        )?;
        let prepared_publication = match self.target.prepare(
            &prepared.effect_identity_sha256,
            &prepared.receipt_name,
            publication_bytes,
        ) {
            Ok(value) => value,
            Err(failure) => {
                return self.finish_terminal_with_timestamp(
                    effect,
                    &prepared.effect_identity_sha256,
                    command_digests,
                    failure.id(),
                    HostEffectState::Ambiguous,
                    completed_at_unix_ms,
                );
            }
        };
        let committed = match self
            .target
            .publish(prepared_publication, effect.record().current_head())
        {
            Ok(committed) => committed,
            Err(failure) => {
                return self.finish_publication_failure(
                    effect,
                    &prepared.effect_identity_sha256,
                    command_digests,
                    completed_at_unix_ms,
                    failure,
                );
            }
        };
        self.acknowledge_reserved_publication(ReservedPublicationAcknowledgement {
            effect,
            effect_identity_sha256: &prepared.effect_identity_sha256,
            command_output_sha256,
            outcome_sha256,
            completed_at_unix_ms,
            committed,
        })
    }
}
