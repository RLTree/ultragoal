pub(crate) struct DescriptorExecutionHandoff {
    capability: DescriptorExecutionCapability,
    effect: AuthorizedHostEffect,
    target: Box<dyn HostTargetLease>,
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

    pub(in crate::distribution::host_effect) fn with_revalidated_observation<R>(
        &self,
        adapter: impl FnOnce(&crate::distribution::host_effect::SelectedCodexExecutable) -> R,
    ) -> Result<R, crate::distribution::host_effect::HostEffectLedgerError> {
        self.revalidate_observation_executable()?;
        Ok(adapter(self.effect.executable()))
    }

    pub(in crate::distribution::host_effect) fn revalidate_observation_executable(
        &self,
    ) -> Result<(), crate::distribution::host_effect::HostEffectLedgerError> {
        self.effect.executable().revalidate()
    }

    pub(in crate::distribution::host_effect) fn recovery_bindings(
        &self,
    ) -> (String, String, String, String) {
        (
            self.effect.permit().permit_id().to_owned(),
            self.effect.plan().plan_sha256().to_owned(),
            self.effect
                .executable()
                .binding_sha256()
                .unwrap_or_default(),
            self.target.identity().target_sha256().to_owned(),
        )
    }

    pub(in crate::distribution::host_effect) fn revalidate_recovery(
        &mut self,
        custody: &crate::plugin_product::lifecycle::HostLifecycleCustody,
        recovery: &crate::distribution::host_effect::HostEffectRecoveryHandoff,
        expected_effect_identity_sha256: &str,
        expected_command_plan_sha256: &str,
        expected_executable_identity_sha256: &str,
        expected_target_identity_sha256: &str,
    ) -> bool {
        let (permit_id, command_plan_sha256, executable_identity_sha256, target_identity_sha256) =
            self.recovery_bindings();
        if permit_id != recovery.permit_id()
            || command_plan_sha256 != expected_command_plan_sha256
            || command_plan_sha256 != custody.plan_sha256()
            || executable_identity_sha256.is_empty()
            || executable_identity_sha256 != expected_executable_identity_sha256
            || target_identity_sha256 != expected_target_identity_sha256
            || recovery.effect_identity_sha256() != expected_effect_identity_sha256
            || !recovery.verify_binding()
        {
            return false;
        }
        self.with_retained_lifecycle(|_, effect, lease, lifecycle| {
            lifecycle.record() == custody.pre_effect_record()
                && effect.plan().plan_sha256() == custody.plan_sha256()
                && effect
                    .executable()
                    .binding_sha256()
                    .is_ok_and(|identity| identity == executable_identity_sha256)
                && lease.identity().target_sha256() == target_identity_sha256
                && lease.revalidate().is_ok()
        })
    }

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

    pub(in crate::distribution::host_effect) fn issue_recovery_disposition(
        &self,
        custody: &mut crate::plugin_product::lifecycle::HostLifecycleCustody,
        recovery: &crate::distribution::host_effect::HostEffectRecoveryHandoff,
    ) -> Result<
        crate::plugin_product::lifecycle::HostLifecycleRecoveryDisposition,
        SupportedHostLifecycleError,
    > {
        let executable_identity_sha256 =
            self.effect.executable().binding_sha256().map_err(|_| {
                lifecycle_error(SupportedHostLifecycleErrorId::ExecutableSubstitution)
            })?;
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

    pub(in crate::distribution::host_effect) fn finalize_recovery(
        self,
        disposition: crate::plugin_product::lifecycle::HostLifecycleRecoveryDisposition,
        recovery: &crate::distribution::host_effect::HostEffectRecoveryHandoff,
    ) -> Result<(), SupportedHostLifecycleError> {
        let executable_identity_sha256 =
            self.effect.executable().binding_sha256().map_err(|_| {
                lifecycle_error(SupportedHostLifecycleErrorId::ExecutableSubstitution)
            })?;
        if !disposition.matches(
            &self.lifecycle_binding,
            self.effect.permit().permit_id(),
            recovery.effect_identity_sha256(),
            self.target.identity().target_sha256(),
            &executable_identity_sha256,
            self.effect.plan().plan_sha256(),
            recovery,
        ) {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::PlanSubstitution,
            ));
        }
        self.effect
            .finalize()
            .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::HandoffConstructionFailed))
    }
}
