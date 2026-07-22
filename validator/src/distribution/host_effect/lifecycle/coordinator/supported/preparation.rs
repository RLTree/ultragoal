struct PreparationContext<'a> {
    accepted: &'a AcceptedHostEffect,
    custody: &'a mut HostLifecycleCustody,
    selected: &'a mut Option<SelectedCodexExecutable>,
    target: &'a mut dyn HostTargetObserver,
    clock: &'a mut dyn RootTrustedClock,
    adapter: &'a mut dyn DescriptorExecutionAdapter,
}

impl<'a> SupportedHostLifecycleCoordinator<'a> {
    fn prepare_selected(
        &self,
        platform: DescriptorExecutionPlatform,
        context: PreparationContext<'_>,
    ) -> Result<DescriptorExecutionHandoff, SupportedHostLifecycleError> {
        let PreparationContext {
            accepted,
            custody,
            selected,
            target,
            clock,
            adapter,
        } = context;
        if !platform.supports_descriptor_execution() {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::DescriptorExecutionUnavailable,
            ));
        }
        accepted.require_coordinator(&self.binding_sha256()?)?;
        accepted.require_plan(custody)?;
        let executable = selected
            .as_ref()
            .ok_or_else(|| lifecycle_error(SupportedHostLifecycleErrorId::PlanSubstitution))?;
        accepted.require_executable(executable)?;
        let capability = adapter.descriptor_capability()?;
        if capability.platform != platform {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::DescriptorExecutionUnavailable,
            ));
        }
        let time_before = clock.sample()?;
        let mut target_lease = target.acquire(accepted.expected_target())?;
        let target_before = target_lease.identity().clone();
        accepted.require_target(&target_before)?;
        let head = self.ledger.ledger.head().map_err(|_| ledger_rejected())?;
        if &head != accepted.expected_head() {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::StaleLedgerHead,
            ));
        }
        let target_after = target_lease.revalidate()?;
        if target_before != target_after {
            return Err(lifecycle_error(SupportedHostLifecycleErrorId::TargetRace));
        }
        accepted.require_target(&target_after)?;
        let time_after = clock.sample()?;
        time_before.require_successor(&time_after)?;
        let expires_at_unix_ms = time_after
            .unix_ms()
            .checked_add(MAX_PERMIT_TTL_MS)
            .ok_or_else(untrusted_time)?;
        let binding = accepted.derive_binding(time_after.unix_ms(), expires_at_unix_ms, &head)?;
        let (permit, reservation) = self
            .authority
            .authority
            .issue(binding)
            .map_err(|_| authority_rejected())?;
        #[cfg(not(test))]
        let _ = &reservation;
        #[cfg(not(test))]
        let verified = self
            .authority
            .authority
            .verify_at(permit, time_after.unix_ms())
            .map_err(|_| authority_rejected())?;
        #[cfg(not(test))]
        let admission = crate::distribution::host_effect::executor::reserve_in_flight_lifecycle(
            self.ledger.ledger,
            verified,
            accepted.lifecycle_record().clone(),
        )
        .map_err(|_| ledger_rejected())?;
        #[cfg(test)]
        self.authority
            .authority
            .verify(&permit, time_after.unix_ms())
            .map_err(|_| authority_rejected())?;
        #[cfg(test)]
        let reserved = self
            .ledger
            .ledger
            .reserve(reservation)
            .map_err(|_| ledger_rejected())?;
        #[cfg(test)]
        let in_flight = self
            .ledger
            .ledger
            .transition(
                HostEffectTransition::new(
                    permit.permit_id().to_owned(),
                    HostEffectState::Reserved,
                    HostEffectState::InFlight,
                    reserved.current_head().clone(),
                    None,
                )
                .map_err(|_| ledger_rejected())?,
            )
            .map_err(|_| ledger_rejected())?;
        let target_final = target_lease.revalidate()?;
        if target_after != target_final {
            return Err(lifecycle_error(SupportedHostLifecycleErrorId::TargetRace));
        }
        accepted.require_target(&target_final)?;
        #[cfg(not(test))]
        let lifecycle_binding = custody
            .begin_effects(&admission)
            .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::PlanSubstitution))?;
        #[cfg(test)]
        let lifecycle_binding = custody.completion_binding();
        #[cfg(not(test))]
        let (permit, in_flight) = admission
            .take_effect_parts()
            .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::LedgerRejected))?;
        #[cfg(not(test))]
        let plan = custody
            .take_command_plan()
            .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::PlanSubstitution))?;
        #[cfg(test)]
        let plan = custody
            .candidate_plan()
            .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::PlanSubstitution))?;
        #[cfg(test)]
        custody
            .commit_release()
            .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::PlanSubstitution))?;
        let executable = selected
            .as_ref()
            .ok_or_else(|| lifecycle_error(SupportedHostLifecycleErrorId::PlanSubstitution))?;
        AuthorizedHostEffect::validate(&permit, &in_flight, executable, &plan).map_err(|_| {
            lifecycle_error(SupportedHostLifecycleErrorId::HandoffConstructionFailed)
        })?;
        let executable = selected
            .take()
            .ok_or_else(|| lifecycle_error(SupportedHostLifecycleErrorId::PlanSubstitution))?;
        let effect = AuthorizedHostEffect::from_validated(permit, in_flight, executable, plan);
        Ok(DescriptorExecutionHandoff {
            capability,
            effect,
            target: target_lease,
            lifecycle_binding,
        })
    }
}
