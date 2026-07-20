/// The only production caller that may keep a durable admission and its ledger
/// together until exact terminal observation is available.
pub(crate) struct IsolatedHostLifecycleAdmission {
    ledger: crate::distribution::host_effect::FileHostEffectLedger,
    admission: DurableHostLifecycleAdmission,
}

pub(crate) fn admit_isolated_lifecycle(
    root: &std::path::Path,
    record: HostLifecycleRecord,
    context_id: &str,
    candidate_id: &str,
    package_sha256: &str,
    host_scope_sha256: &str,
) -> Result<IsolatedHostLifecycleAdmission, ()> {
    let ledger_id = "isolated-install-test-ledger".to_owned();
    let ledger = crate::distribution::host_effect::FileHostEffectLedger::create(
        &root.join("custody"),
        ledger_id.clone(),
    )
    .map_err(|_| ())?;
    let authority = crate::distribution::host_effect::HostEffectAuthority::generate(
        "isolated-install-test".to_owned(),
        ledger_id,
    )
    .map_err(|_| ())?;
    let now = unix_ms()?;
    let binding = isolated_binding(
        record.clone(),
        ledger.head().map_err(|_| ())?,
        context_id,
        candidate_id,
        package_sha256,
        host_scope_sha256,
        now,
    )?;
    let (permit, _) = authority.issue(binding).map_err(|_| ())?;
    let permit = authority.verify_current(permit).map_err(|_| ())?;
    let admission = reserve_in_flight_lifecycle(&ledger, permit, record).map_err(|_| ())?;
    Ok(IsolatedHostLifecycleAdmission { ledger, admission })
}

impl IsolatedHostLifecycleAdmission {
    pub(crate) fn begin_effects(
        &self,
        custody: &mut crate::plugin_product::lifecycle::HostLifecycleCustody,
    ) -> Result<HostEffectExecutionBinding, ()> {
        custody.begin_effects(&self.admission).map_err(|_| ())
    }

    pub(crate) fn settle(
        self,
        custody: &mut crate::plugin_product::lifecycle::HostLifecycleCustody,
        binding: HostEffectExecutionBinding,
        observed: LifecycleState,
        completed_effects: Vec<LifecycleEffect>,
    ) -> Result<(), ()> {
        let completion = self
            .admission
            .settled(&self.ledger, binding, observed, completed_effects)
            .map_err(|_| ())?;
        custody.settle(completion).map_err(|_| ())
    }
}

impl DurableHostLifecycleAdmission {
    fn settled(
        &self,
        ledger: &dyn DurableHostEffectLedger,
        binding: HostEffectExecutionBinding,
        observed: LifecycleState,
        completed_effects: Vec<LifecycleEffect>,
    ) -> Result<HostEffectCompletion, HostEffectLedgerError> {
        if binding.record() != &self.record
            || observed != self.record.expected_after
            || completed_effects != self.record.effects
        {
            return Err(HostEffectLedgerError::new(
                HostEffectLedgerErrorId::InvalidRecord,
            ));
        }
        let outcome_sha256 = serde_json::to_vec(&(&observed, &completed_effects))
            .map(|bytes| format!("sha256:{:x}", Sha256::digest(bytes)))
            .map_err(|_| HostEffectLedgerError::new(HostEffectLedgerErrorId::InvalidRecord))?;
        let settled = ledger.transition(HostEffectTransition::new(
            self._in_flight.reservation().permit_id().to_owned(),
            HostEffectState::InFlight,
            HostEffectState::Settled,
            self._in_flight.current_head().clone(),
            Some(outcome_sha256),
        )?)?;
        if settled.state() != HostEffectState::Settled {
            return Err(HostEffectLedgerError::new(
                HostEffectLedgerErrorId::InvalidRecord,
            ));
        }
        Ok(HostEffectCompletion::settled(
            binding,
            observed,
            completed_effects,
        ))
    }
}

fn isolated_binding(
    record: HostLifecycleRecord,
    head: crate::distribution::host_effect::HostEffectLedgerHead,
    context_id: &str,
    candidate_id: &str,
    package_sha256: &str,
    host_scope_sha256: &str,
    issued_at_unix_ms: u64,
) -> Result<crate::distribution::host_effect::HostEffectPermitBinding, ()> {
    let record_sha256 = digest_json(&record)?;
    let (plan_id, intent) = record.permit_join();
    let joined = digest_json(&(context_id, candidate_id, package_sha256, host_scope_sha256))?;
    Ok(crate::distribution::host_effect::HostEffectPermitBinding {
        context_id: context_id.to_owned(),
        candidate_id: candidate_id.to_owned(),
        package_identity_sha256: package_sha256.to_owned(),
        journey_binding_sha256: joined.clone(),
        session_issuance_sha256: joined.clone(),
        lifecycle_plan_sha256: plan_id.to_owned(),
        lifecycle_intent: intent_name(intent).to_owned(),
        expected_pre_state_sha256: record_sha256.clone(),
        expected_post_state_sha256: record_sha256.clone(),
        rollback_policy_sha256: record_sha256.clone(),
        reconciliation_policy_sha256: record_sha256.clone(),
        host_scope_sha256: host_scope_sha256.to_owned(),
        host_capability_sha256: joined.clone(),
        required_capabilities_sha256: joined.clone(),
        external_request_sha256: joined.clone(),
        command_plan_sha256: record_sha256.clone(),
        argv_sha256: joined.clone(),
        executable_identity_sha256: joined.clone(),
        target_identity_sha256: joined,
        target_generation: 1,
        issued_at_unix_ms,
        expires_at_unix_ms: issued_at_unix_ms.saturating_add(60_000),
        expected_head_sha256: head.head_sha256().to_owned(),
        lifecycle_record: Some(record),
        lifecycle_record_sha256: Some(record_sha256),
        decision: crate::distribution::host_effect::HostEffectDecision::Authorize,
    })
}

fn digest_json(value: &impl serde::Serialize) -> Result<String, ()> {
    serde_json::to_vec(value)
        .map(|bytes| format!("sha256:{:x}", Sha256::digest(bytes)))
        .map_err(|_| ())
}

fn unix_ms() -> Result<u64, ()> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| ())?
        .as_millis()
        .try_into()
        .map_err(|_| ())
}

fn intent_name(intent: crate::plugin_product::lifecycle::LifecycleIntent) -> &'static str {
    use crate::plugin_product::lifecycle::LifecycleIntent;
    match intent {
        LifecycleIntent::FreshInstall => "fresh-install",
        LifecycleIntent::MonotonicUpdate => "monotonic-update",
        LifecycleIntent::FailedUpdateRecovery => "failed-update-recovery",
        LifecycleIntent::AuthorizedRollback => "authorized-rollback",
        LifecycleIntent::IdempotentReinstall => "idempotent-reinstall",
        LifecycleIntent::UninstallTeardown => "uninstall-teardown",
        LifecycleIntent::StaleCacheRecovery => "stale-cache-recovery",
        LifecycleIntent::RepeatUse => "repeat-use",
    }
}
