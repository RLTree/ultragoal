use super::{
    EffectRequest, EffectResolution, EffectSink, EventKind, OrchestrationError, Orchestrator,
};

impl<S: EffectSink> Orchestrator<S> {
    pub fn apply_effect(
        &mut self,
        tick: u64,
        request: EffectRequest,
    ) -> Result<(), OrchestrationError> {
        request.validate()?;
        if request.binding != self.binding {
            return Err(OrchestrationError::StaleBinding);
        }
        if self.journal.is_none()
            || self
                .projection
                .pending_effects
                .contains_key(&request.operation_id)
            || self
                .projection
                .settled_effects
                .contains(&request.operation_id)
        {
            return Err(OrchestrationError::EffectDenied);
        }
        let runtime = self
            .projection
            .leases
            .get(&request.lease_id)
            .ok_or(OrchestrationError::InvalidLease)?;
        if !matches!(runtime.phase, super::replay::LeasePhase::Running)
            || !runtime.spec.owned_scope.effects.contains(&request.effect)
            || tick > runtime.deadline_tick
            || self.projection.root_interrupted
            || self
                .log
                .events()
                .last()
                .is_some_and(|prior| tick < prior.logical_tick)
        {
            return Err(OrchestrationError::EffectDenied);
        }
        let owner = runtime.spec.owner.clone();
        self.append(
            owner.clone(),
            tick,
            EventKind::EffectIntent {
                lease_id: request.lease_id.clone(),
                request: request.clone(),
            },
        )?;
        let receipt = self
            .sink
            .apply(&request)
            .map_err(|_| OrchestrationError::EffectAmbiguous)?;
        receipt
            .validate_for(&request)
            .map_err(|_| OrchestrationError::EffectAmbiguous)?;
        self.append(
            owner,
            tick,
            EventKind::EffectApplied {
                lease_id: request.lease_id,
                receipt,
            },
        )
        .map_err(|_| OrchestrationError::EffectAmbiguous)
    }

    pub fn reconcile_effect(
        &mut self,
        tick: u64,
        lease_id: &str,
        resolution: EffectResolution,
    ) -> Result<(), OrchestrationError> {
        self.append(
            self.root.clone(),
            tick,
            EventKind::EffectReconciled {
                lease_id: lease_id.to_owned(),
                resolution,
            },
        )
    }
}
