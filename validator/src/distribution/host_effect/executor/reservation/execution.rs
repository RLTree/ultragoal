impl<'a> SupportedHostEffectExecutor<'a> {
    /// Private fallible implementation behind `execute_authorized`'s
    /// post-reservation identity guard. No caller may expose this result
    /// directly after the opaque handoff has been consumed.
    fn execute_reserved(
        &mut self,
        capability: &DescriptorExecutionCapability,
        effect: &AuthorizedHostEffect,
        target_lease: &mut dyn HostTargetLease,
        clock: &mut dyn RootTrustedClock,
        cancellation: &HostEffectCancellation,
    ) -> Result<HostEffectExecutionReceipt, HostEffectExecutorFailure> {
        let prepared = match self.prepare_reserved(capability, effect, target_lease, clock)? {
            ReservedExecution::Continue(prepared) => prepared,
            ReservedExecution::Finished(receipt) => return Ok(receipt),
        };
        let command_digests = match self.execute_reserved_commands(
            capability,
            effect,
            &prepared.effect_identity_sha256,
            clock,
            cancellation,
        )? {
            ReservedExecution::Continue(command_digests) => command_digests,
            ReservedExecution::Finished(receipt) => return Ok(receipt),
        };
        self.publish_reserved(effect, prepared, command_digests, clock)
    }
}
