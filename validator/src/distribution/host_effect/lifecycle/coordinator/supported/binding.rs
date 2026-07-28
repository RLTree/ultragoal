impl<'a> SupportedHostLifecycleCoordinator<'a> {
    pub(in crate::distribution::host_effect) fn bind(
        issuer_id: String,
        ledger_id: String,
        ledger: &'a dyn DurableHostEffectLedger,
    ) -> Result<Self, SupportedHostLifecycleError> {
        let authority = BoundAuthority::generate(issuer_id, ledger_id.clone())?;
        let ledger = BoundLedger::bind(ledger, ledger_id)?;
        let coordinator = Self { authority, ledger };
        coordinator.binding_sha256()?;
        Ok(coordinator)
    }

    pub(in crate::distribution::host_effect) fn accept(
        &self,
        request: HostEffectAcceptanceRequest<'_>,
    ) -> Result<AcceptedHostEffect, SupportedHostLifecycleError> {
        AcceptedHostEffect::accept(self.binding_sha256()?, request)
    }

    pub(in crate::distribution::host_effect) fn prepare_current(
        &self,
        request: HostEffectPreparationRequest<'_>,
    ) -> Result<DescriptorExecutionHandoff, SupportedHostLifecycleError> {
        self.prepare_for_platform(DescriptorExecutionPlatform::current(), request)
    }

    fn prepare_for_platform(
        &self,
        platform: DescriptorExecutionPlatform,
        request: HostEffectPreparationRequest<'_>,
    ) -> Result<DescriptorExecutionHandoff, SupportedHostLifecycleError> {
        let HostEffectPreparationRequest {
            accepted,
            custody,
            executable,
            target,
            clock,
            adapter,
        } = request;
        let mut selected = Some(executable);
        let result = self.prepare_selected(
            platform,
            PreparationContext {
                accepted,
                custody,
                selected: &mut selected,
                target,
                clock,
                adapter,
            },
        );
        match result {
            Ok(handoff) => Ok(handoff),
            Err(error) => {
                let cleanup = selected.take().ok_or_else(|| {
                    lifecycle_error(SupportedHostLifecycleErrorId::PlanSubstitution)
                })?;
                Err(preserve_preparation_refusal(error, cleanup.finalize()))
            }
        }
    }

    #[cfg(test)]
    pub(super) fn authorize_recovery(
        &self,
        classification: &PublicationClassification,
        clock: &mut dyn RootTrustedClock,
    ) -> Result<RecoveryAuthorization, SupportedHostLifecycleError> {
        let time = clock.sample()?;
        let head = self.ledger.ledger.head().map_err(|_| ledger_rejected())?;
        issue_recovery_authorization(
            classification,
            &self.binding_sha256()?,
            &head,
            time.unix_ms(),
        )
    }
}

fn preserve_preparation_refusal<E>(
    error: SupportedHostLifecycleError,
    cleanup: Result<(), E>,
) -> SupportedHostLifecycleError {
    let _ = cleanup;
    error
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_failure_does_not_replace_preparation_refusal() {
        let refusal = lifecycle_error(SupportedHostLifecycleErrorId::TargetRace);
        assert_eq!(preserve_preparation_refusal(refusal, Err(())), refusal);
    }
}

pub(in crate::distribution::host_effect) struct HostEffectPreparationRequest<'a> {
    pub accepted: &'a AcceptedHostEffect,
    #[cfg(not(test))]
    pub custody: &'a mut HostLifecycleCustody,
    #[cfg(test)]
    pub custody: &'a mut HostLifecycleCustody,
    pub executable: SelectedCodexExecutable,
    pub target: &'a mut dyn HostTargetObserver,
    pub clock: &'a mut dyn RootTrustedClock,
    pub adapter: &'a mut dyn DescriptorExecutionAdapter,
}

include!("preparation.rs");
