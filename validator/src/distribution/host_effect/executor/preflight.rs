impl<'a> SupportedHostEffectExecutor<'a> {
    fn preflight(
        &self,
        effect: &AuthorizedHostEffect,
        target_lease: &mut dyn HostTargetLease,
    ) -> Result<(), HostEffectExecutorFailure> {
        let current = self
            .ledger
            .read(effect.permit().permit_id())
            .map_err(|_| ledger_failure())?;
        match current {
            Some(record) if &record == effect.record() => {}
            Some(record)
                if matches!(
                    record.state(),
                    HostEffectState::Settled | HostEffectState::Failed | HostEffectState::Ambiguous
                ) =>
            {
                return Err(HostEffectExecutorFailure::new(
                    HostEffectExecutorErrorId::Replay,
                ));
            }
            _ => return Err(ledger_failure()),
        }
        if effect.record().state() != HostEffectState::InFlight
            || self.ledger.head().map_err(|_| ledger_failure())? != *effect.record().current_head()
        {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::LedgerRollback,
            ));
        }
        self.target.revalidate_anchor()?;
        let lease_before = target_lease.identity().clone();
        if &lease_before != self.target.expected_target() {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::TargetSubstitution,
            ));
        }
        let lease_after = target_lease.revalidate().map_err(|_| target_failure())?;
        if lease_before != lease_after || &lease_after != self.target.expected_target() {
            return Err(target_failure());
        }
        effect.executable().revalidate().map_err(|_| {
            HostEffectExecutorFailure::new(HostEffectExecutorErrorId::ExecutableMutation)
        })?;
        Ok(())
    }

    fn effect_identity(
        &self,
        capability: &DescriptorExecutionCapability,
        effect: &AuthorizedHostEffect,
    ) -> Result<String, HostEffectExecutorFailure> {
        #[derive(Serialize)]
        struct Identity<'a> {
            schema: &'static str,
            permit_id: &'a str,
            permit_binding_sha256: &'a str,
            semantic_key_sha256: &'a str,
            command_plan_sha256: &'a str,
            executable_identity_sha256: String,
            target_identity_sha256: &'a str,
            in_flight_ledger_head: &'a HostEffectLedgerHead,
            descriptor_capability_sha256: &'a str,
            environment_sha256: &'a str,
            execution_policy_sha256: &'a str,
        }
        digest_json(&Identity {
            schema: "harness-ultragoal.authorized-host-effect-execution.v1",
            permit_id: effect.permit().permit_id(),
            permit_binding_sha256: effect.permit().binding_sha256(),
            semantic_key_sha256: effect.permit().semantic_key_sha256(),
            command_plan_sha256: effect.plan().plan_sha256(),
            executable_identity_sha256: effect.executable().identity().binding_sha256().map_err(
                |_| HostEffectExecutorFailure::new(HostEffectExecutorErrorId::ExecutableMutation),
            )?,
            target_identity_sha256: self.target.expected_target().target_sha256(),
            in_flight_ledger_head: effect.record().current_head(),
            descriptor_capability_sha256: capability.capability_sha256(),
            environment_sha256: self.policy.environment_sha256(),
            execution_policy_sha256: self.policy.policy_sha256(),
        })
    }

    fn has_current_in_flight_record(&self, effect: &AuthorizedHostEffect) -> bool {
        effect.record().state() == HostEffectState::InFlight
            && self
                .ledger
                .read(effect.permit().permit_id())
                .ok()
                .flatten()
                .is_some_and(|current| current == *effect.record())
            && self
                .ledger
                .head()
                .is_ok_and(|head| head == *effect.record().current_head())
    }

    fn transition_terminal(
        &self,
        effect: &AuthorizedHostEffect,
        state: HostEffectState,
        outcome_sha256: String,
    ) -> Result<HostEffectLedgerRecord, HostEffectExecutorFailure> {
        let before = self
            .ledger
            .read(effect.permit().permit_id())
            .map_err(|_| ledger_failure())?
            .ok_or_else(ledger_failure)?;
        if before.state() != HostEffectState::InFlight
            || before.reservation() != effect.record().reservation()
            || self.ledger.head().map_err(|_| ledger_failure())? != *before.current_head()
        {
            return Err(ledger_failure());
        }
        let terminal = self
            .ledger
            .transition(
                HostEffectTransition::new(
                    effect.permit().permit_id().to_owned(),
                    HostEffectState::InFlight,
                    state,
                    before.current_head().clone(),
                    Some(outcome_sha256),
                )
                .map_err(|_| ledger_failure())?,
            )
            .map_err(|_| ledger_failure())?;
        if terminal.state() != state
            || terminal.reservation() != effect.record().reservation()
            || self.ledger.head().map_err(|_| ledger_failure())? != *terminal.current_head()
            || self
                .ledger
                .read(effect.permit().permit_id())
                .map_err(|_| ledger_failure())?
                != Some(terminal.clone())
        {
            return Err(ledger_failure());
        }
        Ok(terminal)
    }
}
