pub(crate) struct DescriptorExecutionHandoff {
    capability: DescriptorExecutionCapability,
    effect: AuthorizedHostEffect,
    target: Box<dyn HostTargetLease>,
    #[cfg(not(test))]
    lifecycle_binding: crate::plugin_product::lifecycle::HostEffectExecutionBinding,
}

impl std::fmt::Debug for DescriptorExecutionHandoff {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DescriptorExecutionHandoff")
            .field("platform", &self.capability.platform)
            .field("primitive", &self.capability.primitive)
            .finish_non_exhaustive()
    }
}

impl DescriptorExecutionHandoff {
    #[cfg(test)]
    pub(in crate::distribution::host_effect) fn with_retained_authority<R>(
        mut self,
        adapter: impl FnOnce(
            &DescriptorExecutionCapability,
            &AuthorizedHostEffect,
            &mut dyn HostTargetLease,
        ) -> R,
    ) -> R {
        adapter(&self.capability, &self.effect, self.target.as_mut())
    }

    #[cfg(test)]
    pub(in crate::distribution::host_effect) fn with_retained_authority_mut<R>(
        &mut self,
        adapter: impl FnOnce(
            &DescriptorExecutionCapability,
            &AuthorizedHostEffect,
            &mut dyn HostTargetLease,
        ) -> R,
    ) -> R {
        adapter(&self.capability, &self.effect, self.target.as_mut())
    }

    #[cfg(not(test))]
    pub(in crate::distribution::host_effect) fn with_retained_lifecycle<R>(
        &mut self,
        adapter: impl FnOnce(
            &DescriptorExecutionCapability,
            &AuthorizedHostEffect,
            &mut dyn HostTargetLease,
            &crate::plugin_product::lifecycle::HostEffectExecutionBinding,
        ) -> R,
    ) -> R {
        adapter(
            &self.capability,
            &self.effect,
            self.target.as_mut(),
            &self.lifecycle_binding,
        )
    }

    #[cfg(not(test))]
    pub(in crate::distribution::host_effect) fn finalize(
        self,
        finalization: crate::plugin_product::lifecycle::HostLifecycleFinalization,
    ) -> Result<(), SupportedHostLifecycleError> {
        if !finalization.matches(&self.lifecycle_binding) {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::PlanSubstitution,
            ));
        }
        self.effect
            .finalize()
            .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::HandoffConstructionFailed))
    }

    #[cfg(not(test))]
    pub(in crate::distribution::host_effect) fn issue_recovery_disposition(
        &self,
        custody: &mut crate::plugin_product::lifecycle::HostLifecycleCustody,
        recovery: &crate::distribution::host_effect::HostEffectRecoveryHandoff,
    ) -> Result<
        crate::plugin_product::lifecycle::HostLifecycleRecoveryDisposition,
        SupportedHostLifecycleError,
    > {
        let executable_identity_sha256 = self
            .effect
            .executable()
            .binding_sha256()
            .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::ExecutableSubstitution))?;
        custody
            .issue_recovery_disposition(
                &self.lifecycle_binding,
                self.effect.permit().permit_id().to_owned(),
                recovery.effect_identity_sha256().to_owned(),
                self.target.identity().target_sha256().to_owned(),
                executable_identity_sha256,
                recovery,
            )
            .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::RecoveryUnsafe))
    }

    #[cfg(not(test))]
    pub(in crate::distribution::host_effect) fn finalize_recovery(
        self,
        disposition: crate::plugin_product::lifecycle::HostLifecycleRecoveryDisposition,
        recovery: &crate::distribution::host_effect::HostEffectRecoveryHandoff,
    ) -> Result<(), SupportedHostLifecycleError> {
        let executable_identity_sha256 = self
            .effect
            .executable()
            .binding_sha256()
            .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::ExecutableSubstitution))?;
        if !disposition.matches(
            &self.lifecycle_binding,
            self.effect.permit().permit_id(),
            recovery.effect_identity_sha256(),
            self.target.identity().target_sha256(),
            &executable_identity_sha256,
            self.effect.plan().plan_sha256(),
            recovery,
        ) {
            return Err(lifecycle_error(SupportedHostLifecycleErrorId::PlanSubstitution));
        }
        self.effect
            .finalize()
            .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::HandoffConstructionFailed))
    }

    #[cfg(not(test))]
    pub(in crate::distribution::host_effect) fn retain_for_recovery(self) {
        std::mem::forget(self);
    }

    #[cfg(test)]
    pub(in crate::distribution::host_effect) fn finalize(
        self,
        _finalization: crate::plugin_product::lifecycle::HostLifecycleFinalization,
    ) -> Result<(), SupportedHostLifecycleError> {
        self.effect
            .finalize()
            .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::HandoffConstructionFailed))
    }
}
