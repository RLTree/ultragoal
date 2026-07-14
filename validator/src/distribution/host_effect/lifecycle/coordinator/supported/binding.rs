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
        // This must remain the first observable decision. In particular,
        // Darwin may not contact clocks, targets, ledgers, permit authority, or
        // descriptor adapters and may not release the plan.
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
        self.authority
            .authority
            .verify(&permit, time_after.unix_ms())
            .map_err(|_| authority_rejected())?;
        let reserved = self
            .ledger
            .ledger
            .reserve(reservation)
            .map_err(|_| ledger_rejected())?;
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
        // Construct against a non-authoritative plan copy so a final
        // executable revalidation failure cannot consume root plan custody.
        let plan = custody.candidate_plan()?;
        let effect =
            AuthorizedHostEffect::new(permit, in_flight, executable, plan).map_err(|_| {
                lifecycle_error(SupportedHostLifecycleErrorId::HandoffConstructionFailed)
            })?;
        custody.commit_release()?;
        Ok(DescriptorExecutionHandoff {
            capability,
            effect,
            target: target_lease,
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
    pub custody: &'a mut RootPlanCustody,
    pub executable: PinnedHostExecutable,
    pub target: &'a mut dyn HostTargetObserver,
    pub clock: &'a mut dyn RootTrustedClock,
    pub adapter: &'a mut dyn DescriptorExecutionAdapter,
}
