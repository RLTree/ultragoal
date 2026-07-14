use super::*;

impl ProductionRoutineIssuer {
    /// Opens an explicit, pre-existing, owner-only authority root. This is an
    /// effectful constructor and must never be called by help/read/query paths.
    pub(crate) fn open(authority_root: &Path) -> Result<Self, RoutineError> {
        FileAuthorityLedger::open_or_initialize(authority_root).map(|ledger| Self {
            ledger: Arc::new(ledger),
        })
    }

    pub(crate) fn open_existing(authority_root: &Path) -> Result<Self, RoutineError> {
        FileAuthorityLedger::open_existing(authority_root).map(|ledger| Self {
            ledger: Arc::new(ledger),
        })
    }

    /// Reconstructs bounded recovery authority only for the exact current
    /// request binding and a durable reserved-or-started record.
    pub(crate) fn pending_recovery(
        &self,
        context: &LiveContext,
        plan: &RoutinePlan,
        prepared: &PreparedRoutineExecution,
    ) -> Result<Option<RoutineRecoveryAuthority>, RoutineError> {
        let PreparedRoutineExecution::Effect(request) = prepared else {
            return Ok(None);
        };
        preflight_production_request(context, plan, request)?;
        let binding = authority_binding(request)?;
        self.ledger.pending_recovery(&binding).map(|pending| {
            pending.map(|pending| RoutineRecoveryAuthority {
                binding,
                marker: pending.marker,
                deadline_tick: pending.deadline_tick,
            })
        })
    }

    pub(crate) fn mediate(
        &self,
        context: &LiveContext,
        plan: &RoutinePlan,
        prepared: PreparedRoutineExecution,
        recovery: Option<RoutineRecoveryAuthority>,
        cancellation: RoutineCancellation,
        reuse: RoutineReuseInput,
    ) -> Result<RoutineMediationResult, RoutineError> {
        let PreparedRoutineExecution::Effect(request) = prepared else {
            if recovery.is_some() || !reuse.is_empty() {
                return Err(error("routine-production-noop-authority-or-reuse-present"));
            }
            return mediate_prepared_routine_execution(
                context,
                plan,
                prepared,
                None,
                cancellation,
                reuse,
            );
        };
        preflight_production_request(context, plan, &request)?;
        let require_complete_reuse_set = recovery.is_none() && !reuse.is_empty();
        let reuse = preflight_production_reuse_input(reuse, &request, require_complete_reuse_set)?;
        self.mediate_preflighted(context, plan, request, recovery, cancellation, reuse)
    }

    pub(crate) fn mediate_preflighted(
        &self,
        context: &LiveContext,
        plan: &RoutinePlan,
        request: RoutineEffectRequest,
        recovery: Option<RoutineRecoveryAuthority>,
        cancellation: RoutineCancellation,
        reuse: PreflightedProductionReuse,
    ) -> Result<RoutineMediationResult, RoutineError> {
        let (reuse, reuse_claims) = reuse.into_parts();
        let authority_binding = authority_binding(&request)?;
        let recovery_for = match recovery {
            Some(recovery)
                if recovery.binding == authority_binding
                    && now_tick()? <= recovery.deadline_tick =>
            {
                Some(recovery.marker)
            }
            Some(_) => return Err(error("routine-production-recovery-authority-stale")),
            None => None,
        };
        let reuse_only = recovery_for.is_none() && !reuse.is_empty();
        if !reuse_claims.is_empty() && !reuse_only {
            return Err(error("mediator-production-reuse-not-authenticated"));
        }
        let reuse_preauthorization = if reuse_only {
            let claims = reuse_claims
                .into_iter()
                .map(|claim| ReuseArtifactClaim {
                    protocol_id: claim.protocol_id,
                    intent_id: claim.intent_id,
                    artifact_sha256: claim.artifact_sha256,
                    result_artifact_sha256: claim.result_artifact_sha256,
                    mediator_witness_sha256: claim.mediator_witness_sha256,
                })
                .collect();
            let authorization = self.ledger.preauthorize_reuse(&authority_binding, claims)?;
            run_test_reuse_preauthorization_hook();
            Some(authorization)
        } else {
            None
        };
        let session_id = random_session_id(&authority_binding)?;
        let allowed_output_scopes = allowed_output_scopes(&request);
        let grant_binding = ProductionGrantBinding {
            session_id,
            request_id: request.request_id().to_owned(),
            protocol_id: request.protocol_id().to_owned(),
            context_id: request.context_id().to_owned(),
            candidate_id: request.candidate_id().to_owned(),
            plan_id: request.plan_id().to_owned(),
            snapshot_id: authority_binding.snapshot_id.clone(),
            allowed_output_scopes,
            recovery_for: recovery_for.clone(),
        };
        let grant_id = production_grant_identity(&grant_binding)?;
        let recovery_marker =
            production_recovery_identity(&grant_id, request.protocol_id(), request.request_id());
        let token = self.ledger.reserve(ReservationSpec {
            binding: authority_binding,
            request_id: request.request_id().to_owned(),
            grant_id,
            recovery_marker,
            recovery_for,
            reuse_only,
            reuse_preauthorization,
        })?;
        let durable = Arc::new(DurableAttempt {
            ledger: Arc::clone(&self.ledger),
            token,
        });
        let grant = issue_production_grant(grant_binding, durable)?;
        mediate_prepared_routine_execution(
            context,
            plan,
            PreparedRoutineExecution::Effect(request),
            Some(grant),
            cancellation,
            reuse,
        )
    }

    #[cfg(test)]
    pub(crate) fn test_set_reuse_preauthorization_hook(hook: impl FnOnce() + Send + 'static) {
        REUSE_PREAUTHORIZATION_TEST_HOOK.with(|slot| {
            assert!(slot.borrow_mut().replace(Box::new(hook)).is_none());
        });
    }

    #[cfg(test)]
    pub(crate) fn test_reserve_and_abandon(
        &self,
        context: &LiveContext,
        plan: &RoutinePlan,
        prepared: &PreparedRoutineExecution,
        started: bool,
    ) -> Result<(), RoutineError> {
        let PreparedRoutineExecution::Effect(request) = prepared else {
            return Err(error("routine-production-test-effect-required"));
        };
        preflight_production_request(context, plan, request)?;
        let authority_binding = authority_binding(request)?;
        let grant_binding = ProductionGrantBinding {
            session_id: random_session_id(&authority_binding)?,
            request_id: request.request_id().to_owned(),
            protocol_id: request.protocol_id().to_owned(),
            context_id: request.context_id().to_owned(),
            candidate_id: request.candidate_id().to_owned(),
            plan_id: request.plan_id().to_owned(),
            snapshot_id: authority_binding.snapshot_id.clone(),
            allowed_output_scopes: allowed_output_scopes(request),
            recovery_for: None,
        };
        let grant_id = production_grant_identity(&grant_binding)?;
        let recovery_marker =
            production_recovery_identity(&grant_id, request.protocol_id(), request.request_id());
        let token = self.ledger.reserve(ReservationSpec {
            binding: authority_binding,
            request_id: request.request_id().to_owned(),
            grant_id,
            recovery_marker,
            recovery_for: None,
            reuse_only: false,
            reuse_preauthorization: None,
        })?;
        if started {
            self.ledger.prepare_spawn(&token)?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn test_expire_pending(
        &self,
        context: &LiveContext,
        plan: &RoutinePlan,
        prepared: &PreparedRoutineExecution,
    ) -> Result<(), RoutineError> {
        let PreparedRoutineExecution::Effect(request) = prepared else {
            return Err(error("routine-production-test-effect-required"));
        };
        preflight_production_request(context, plan, request)?;
        self.ledger
            .test_expire_pending(&authority_binding(request)?)
    }

    #[cfg(test)]
    pub(crate) fn test_seed_capacity(
        &self,
        protocol_effect_count: usize,
        consumed_grant_count: usize,
    ) -> Result<(), RoutineError> {
        self.ledger
            .test_seed_capacity(protocol_effect_count, consumed_grant_count)
    }

    #[cfg(test)]
    pub(crate) const fn test_capacity_limits() -> (usize, usize) {
        FileAuthorityLedger::test_capacity_limits()
    }
}
