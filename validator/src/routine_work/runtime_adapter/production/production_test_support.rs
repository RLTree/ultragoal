use super::*;

impl ProductionRoutineIssuer {
    pub(crate) fn test_set_reuse_preauthorization_hook(hook: impl FnOnce() + Send + 'static) {
        REUSE_PREAUTHORIZATION_TEST_HOOK.with(|slot| {
            assert!(slot.borrow_mut().replace(Box::new(hook)).is_none());
        });
    }

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

    pub(crate) fn test_seed_capacity(
        &self,
        protocol_effect_count: usize,
        consumed_grant_count: usize,
    ) -> Result<(), RoutineError> {
        self.ledger
            .test_seed_capacity(protocol_effect_count, consumed_grant_count)
    }

    pub(crate) const fn test_capacity_limits() -> (usize, usize) {
        FileAuthorityLedger::test_capacity_limits()
    }
}
