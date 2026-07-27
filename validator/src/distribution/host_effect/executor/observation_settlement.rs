impl<'a> SupportedHostEffectExecutor<'a> {
    pub(in crate::distribution::host_effect) fn settle_observation(
        &self,
        capability: &DescriptorExecutionCapability,
        effect: &AuthorizedHostEffect,
        target_lease: &mut dyn HostTargetLease,
        clock: &mut dyn RootTrustedClock,
    ) -> Result<(), HostEffectExecutorFailure> {
        self.preflight(effect, target_lease)?;
        let identity = self.effect_identity(capability, effect)?;
        let completed_at_unix_ms = clock
            .sample()
            .map_err(|_| HostEffectExecutorFailure::new(HostEffectExecutorErrorId::Io))?
            .unix_ms();
        let outcome_sha256 = outcome_digest(
            &identity,
            effect,
            HostEffectState::Settled,
            &[] as &[CommandCaptureDigest],
            self.policy.policy_sha256(),
            completed_at_unix_ms,
        )?;
        self.transition_terminal(effect, HostEffectState::Settled, outcome_sha256)?;
        Ok(())
    }
}
