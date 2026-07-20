#[derive(Clone, Copy)]
struct PriorPublicationEvidence<'a> {
    publication_identity_sha256: Option<&'a str>,
    observation: Option<&'a PublicationInventoryObservation>,
}

struct PostReservationLedgerEvidence {
    ledger_head: HostEffectLedgerHead,
    ledger_record: HostEffectLedgerRecord,
    exact_current_observation: bool,
    classification: HostEffectPostReservationLedgerClassification,
    terminal_state: Option<HostEffectState>,
    observation_error_id: Option<HostEffectExecutorErrorId>,
}

pub(crate) struct SupportedHostEffectExecutor<'a> {
    ledger: &'a dyn DurableHostEffectLedger,
    target: ConfinedHostEffectTarget,
    backend: &'a mut dyn RetainedDescriptorProcessBackend,
    policy: HostEffectExecutionPolicy,
}

impl std::fmt::Debug for SupportedHostEffectExecutor<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SupportedHostEffectExecutor")
            .field("target", &self.target)
            .field("policy_sha256", &self.policy.policy_sha256())
            .finish_non_exhaustive()
    }
}

impl<'a> SupportedHostEffectExecutor<'a> {
    pub(in crate::distribution::host_effect) fn new(
        ledger: &'a dyn DurableHostEffectLedger,
        target: ConfinedHostEffectTarget,
        backend: &'a mut dyn RetainedDescriptorProcessBackend,
        policy: HostEffectExecutionPolicy,
    ) -> Self {
        Self {
            ledger,
            target,
            backend,
            policy,
        }
    }

    /// Consumes the accepted opaque handoff while it retains the exact target
    /// lease and authorized effect. No `AuthorizedHostEffect` escapes this
    /// synchronous call.
    pub(in crate::distribution::host_effect) fn execute_handoff(
        &mut self,
        handoff: DescriptorExecutionHandoff,
        clock: &mut dyn RootTrustedClock,
        cancellation: &HostEffectCancellation,
    ) -> Result<HostEffectExecutionReceipt, HostEffectExecutorFailure> {
        handoff.with_retained_authority(|capability, effect, lease| {
            if capability.platform() != DescriptorExecutionPlatform::current() {
                let effect_identity_sha256 = self
                    .effect_identity(capability, effect)
                    .unwrap_or_else(|_| fallback_effect_identity(effect));
                return Err(self.post_reservation_recovery_failure(
                    effect,
                    &effect_identity_sha256,
                    HostEffectExecutorErrorId::UnsupportedPlatform,
                    vec![HostEffectExecutorErrorId::UnsupportedPlatform],
                    None,
                ));
            }
            self.execute_authorized(capability, effect, lease, clock, cancellation)
        })
    }

    fn execute_authorized(
        &mut self,
        capability: &DescriptorExecutionCapability,
        effect: &AuthorizedHostEffect,
        target_lease: &mut dyn HostTargetLease,
        clock: &mut dyn RootTrustedClock,
        cancellation: &HostEffectCancellation,
    ) -> Result<HostEffectExecutionReceipt, HostEffectExecutorFailure> {
        let boundary_effect_identity = self
            .effect_identity(capability, effect)
            .unwrap_or_else(|_| fallback_effect_identity(effect));
        match self.execute_reserved(capability, effect, target_lease, clock, cancellation) {
            Err(failure) if failure.recovery().is_none() => Err(self
                .post_reservation_recovery_failure(
                    effect,
                    &boundary_effect_identity,
                    failure.id(),
                    vec![failure.id()],
                    None,
                )),
            result => result,
        }
    }
}
