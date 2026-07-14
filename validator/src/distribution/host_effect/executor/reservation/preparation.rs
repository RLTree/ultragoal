struct PreparedHostEffectExecution {
    effect_identity_sha256: String,
    receipt_name: String,
}

enum ReservedExecution<T> {
    Continue(T),
    Finished(HostEffectExecutionReceipt),
}

fn terminal_execution<T>(
    result: Result<HostEffectExecutionReceipt, HostEffectExecutorFailure>,
) -> Result<ReservedExecution<T>, HostEffectExecutorFailure> {
    result.map(ReservedExecution::Finished)
}

impl<'a> SupportedHostEffectExecutor<'a> {
    fn prepare_reserved(
        &mut self,
        capability: &DescriptorExecutionCapability,
        effect: &AuthorizedHostEffect,
        target_lease: &mut dyn HostTargetLease,
        clock: &mut dyn RootTrustedClock,
    ) -> Result<ReservedExecution<PreparedHostEffectExecution>, HostEffectExecutorFailure> {
        if let Err(failure) = self.preflight(effect, target_lease) {
            if self.has_current_in_flight_record(effect) {
                return terminal_execution(self.finish_backend_failure(
                    effect,
                    &fallback_effect_identity(effect),
                    Vec::new(),
                    BackendFailure::before_start(failure.id()),
                    clock,
                ));
            }
            if failure.id() == HostEffectExecutorErrorId::LedgerSubstitution {
                let effect_identity_sha256 = self
                    .effect_identity(capability, effect)
                    .unwrap_or_else(|_| fallback_effect_identity(effect));
                return Err(self.identity_bearing_preflight_ledger_failure(
                    effect,
                    &effect_identity_sha256,
                    failure.id(),
                ));
            }
            return Err(failure);
        }
        let effect_identity_sha256 = match self.effect_identity(capability, effect) {
            Ok(identity) => identity,
            Err(failure) => {
                return terminal_execution(self.finish_backend_failure(
                    effect,
                    &fallback_effect_identity(effect),
                    Vec::new(),
                    BackendFailure::before_start(failure.id()),
                    clock,
                ));
            }
        };
        let receipt_name = receipt_name(effect.permit().permit_id())?;
        if let Err(failure) = self.target.require_clean_publication_name(&receipt_name) {
            return terminal_execution(self.finish_backend_failure(
                effect,
                &effect_identity_sha256,
                Vec::new(),
                BackendFailure::before_start(failure.id()),
                clock,
            ));
        }
        Ok(ReservedExecution::Continue(PreparedHostEffectExecution {
            effect_identity_sha256,
            receipt_name,
        }))
    }
}
