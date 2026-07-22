#[cfg(not(test))]
impl HostLifecycleCustody {
    pub(crate) fn issue_recovery_disposition(
        &mut self,
        binding: &HostEffectExecutionBinding,
        permit_id: String,
        effect_identity_sha256: String,
        target_identity_sha256: String,
        executable_identity_sha256: String,
        recovery: &HostEffectRecoveryHandoff,
    ) -> Result<HostLifecycleRecoveryDisposition, LifecycleError> {
        if !matches!(self.custody_phase, 1 | 2) || binding.record() != &self.pre_effect_record {
            return Err(LifecycleError::InvalidTransition);
        }
        if !recovery.verify_binding()
            || recovery.permit_id() != permit_id
            || recovery.disposition_state().is_none()
        {
            return Err(LifecycleError::RecoveryUnavailable);
        }
        let terminal_state = recovery
            .disposition_state()
            .ok_or(LifecycleError::RecoveryUnavailable)?;
        Ok(HostLifecycleRecoveryDisposition {
            record: self.pre_effect_record.clone(),
            permit_id,
            effect_identity_sha256,
            target_identity_sha256,
            executable_identity_sha256,
            command_plan_sha256: self.pre_effect_record.command_plan_sha256().to_owned(),
            recovery_binding_sha256: recovery.disposition_binding_sha256().to_owned(),
            terminal_state,
        })
    }
}
