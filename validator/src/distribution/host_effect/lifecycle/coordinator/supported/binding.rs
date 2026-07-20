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
        #[cfg(test)]
        if platform == DescriptorExecutionPlatform::Darwin {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::UnsupportedPlatform,
            ));
        }
        if !platform.supports_descriptor_execution() {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::DescriptorExecutionUnavailable,
            ));
        }

        accepted.require_coordinator(&self.binding_sha256()?)?;
        accepted.require_plan(custody)?;
        accepted.require_executable(&executable)?;
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
        let effect =
            AuthorizedHostEffect::new(permit, in_flight, executable, plan).map_err(|_| {
                lifecycle_error(SupportedHostLifecycleErrorId::HandoffConstructionFailed)
            })?;
        Ok(DescriptorExecutionHandoff {
            capability,
            effect,
            target: target_lease,
            #[cfg(not(test))]
            lifecycle_binding,
        })
    }

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

pub(in crate::distribution::host_effect) struct HostEffectPreparationRequest<'a> {
    pub accepted: &'a AcceptedHostEffect,
    #[cfg(not(test))]
    pub custody: &'a mut HostLifecycleCustody,
    #[cfg(test)]
    pub custody: &'a mut HostLifecycleCustody,
    pub executable: PinnedHostExecutable,
    pub target: &'a mut dyn HostTargetObserver,
    pub clock: &'a mut dyn RootTrustedClock,
    pub adapter: &'a mut dyn DescriptorExecutionAdapter,
}
