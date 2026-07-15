use super::binding::ReservationBinding;
use super::durable_binding::DurableBinding;
use super::read_source_binding::release_active;
use super::registry_transition;
use super::staged_custody::StagedCustody;
use super::*;
use std::cell::Cell;

pub(in super::super) struct AttemptReservation {
    binding: ReservationBinding,
    started: Cell<bool>,
    settled: Cell<bool>,
    durable: DurableBinding,
    staged: StagedCustody,
}

impl AttemptReservation {
    pub(in super::super) fn reserved(
        protocol_id: String,
        grant_id: String,
        recovery_marker: String,
        prior_recovery_marker: Option<String>,
        durable: Option<Arc<dyn DurableAttemptAuthority>>,
    ) -> Self {
        Self {
            binding: ReservationBinding::new(
                protocol_id,
                grant_id,
                recovery_marker,
                prior_recovery_marker,
            ),
            started: Cell::new(false),
            settled: Cell::new(false),
            durable: DurableBinding::new(durable),
            staged: StagedCustody::new(),
        }
    }

    #[cfg(test)]
    pub(in super::super) fn protocol_id(&self) -> &String {
        self.binding.protocol_id()
    }

    #[cfg(test)]
    pub(in super::super) fn grant_id(&self) -> &String {
        self.binding.grant_id()
    }

    #[cfg(test)]
    pub(in super::super) fn recovery_marker(&self) -> &String {
        self.binding.recovery_marker()
    }

    pub(in super::super) fn is_started(&self) -> bool {
        self.started.get()
    }

    pub(super) fn terminal_is_authoritative(&self) -> bool {
        self.settled.get() && self.staged.is_empty()
    }

    fn require_open(&self) -> Result<(), RoutineError> {
        if self.settled.get() {
            return Err(mediator_error(
                "mediator-reservation-terminal-already-settled",
            ));
        }
        Ok(())
    }

    pub(in super::super) fn reuse_only(&self) -> bool {
        self.durable.reuse_only()
    }

    pub(in super::super) fn prepare_spawn(&self) -> Result<(), RoutineError> {
        self.require_open()?;
        self.durable.prepare_spawn()
    }

    pub(in super::super) fn mark_started(&self) -> Result<(), RoutineError> {
        self.require_open()?;
        registry_transition::mark_started(&self.binding)?;
        self.started.set(true);
        Ok(())
    }

    pub(in super::super) fn stage_success(
        &self,
        artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        self.require_open()?;
        self.staged.require_empty()?;
        self.durable.stage_success(artifacts)
    }

    pub(in super::super) fn settle_success(
        &self,
        artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        self.require_open()?;
        self.staged.require_empty()?;
        self.durable
            .settle(DurableSettlement::Complete, artifacts)?;
        self.finish_terminal(true);
        Ok(())
    }

    pub(in super::super) fn settle_incomplete(
        &self,
        outcome: DurableSettlement,
    ) -> Result<Option<String>, RoutineError> {
        self.require_open()?;
        self.staged.require_empty()?;
        let durable = self.durable.settle(outcome, &BTreeMap::new())?;
        let pending = self.finish_terminal(durable);
        Ok((!durable).then_some(pending).flatten())
    }

    fn finish_terminal(&self, durable: bool) -> Option<String> {
        let pending =
            registry_transition::finish_terminal(&self.binding, self.started.get(), durable);
        self.settled.set(true);
        pending
    }

    pub(in super::super) fn retain_non_durable_authentication(
        &self,
        artifacts: &BTreeMap<String, String>,
    ) {
        if !self.durable.is_present() {
            registry_transition::retain_non_durable(artifacts);
        }
    }

    pub(in super::super) fn authenticates_artifact(
        &self,
        digest: &str,
        witness: &str,
    ) -> Result<bool, RoutineError> {
        if let Some(result) = self.durable.authenticates_artifact(digest, witness)? {
            return Ok(result);
        }
        Ok(registry_transition::authenticates_non_durable(
            digest, witness,
        ))
    }

    pub(in super::super) fn stage_and_use<T>(
        &self,
        program: &PinnedExecutable,
        use_program: impl FnOnce(&PinnedExecutable) -> Result<T, RoutineError>,
    ) -> Result<T, RoutineError> {
        self.require_open()?;
        self.staged.require_empty()?;
        let staged = self.durable.stage_program(program)?;
        self.require_open()?;
        self.staged.require_empty()?;
        self.staged.push_and_use(staged, use_program)
    }

    pub(super) fn cleanup_staged(&self) -> Result<(), RoutineError> {
        if !self.durable.is_present() {
            return if self.staged.is_empty() {
                Ok(())
            } else {
                Err(mediator_error("mediator-staging-authority-missing"))
            };
        }
        while self
            .staged
            .cleanup_last(|staged| self.durable.cleanup_staged(staged))?
        {}
        Ok(())
    }

    pub(super) fn failure_evidence(
        &self,
        primary: FailureEvidence,
        process_cleanup: CleanupEvidence,
        staged_cleanup: CleanupEvidence,
    ) -> ReservationFailureEvidence {
        self.binding
            .failure_evidence(primary, process_cleanup, staged_cleanup, self.started.get())
    }

    pub(super) fn record_failure_and_transition(
        &self,
        evidence: &ReservationFailureEvidence,
    ) -> Result<(), RoutineError> {
        self.require_open()?;
        if !self.binding.validates_failure(evidence, self.started.get()) {
            return Err(mediator_error(
                "mediator-reservation-failure-evidence-binding-invalid",
            ));
        }
        let transfer = self
            .staged
            .failure_transfer_required(&evidence.staged_cleanup, self.durable.is_present())?;
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.active_protocols.get(self.binding.protocol_id()) != Some(self.binding.grant_id()) {
            return Err(mediator_error(
                "mediator-reservation-failure-active-binding-invalid",
            ));
        }
        if !self.durable.record_failure(evidence)? {
            registry_transition::record_non_durable_failure(&mut state, &self.binding, evidence)?;
        }
        if transfer {
            self.staged.clear_recorded();
        }
        release_active(
            &mut state,
            self.binding.protocol_id(),
            self.binding.grant_id(),
        );
        if self.started.get()
            && !state
                .ambiguous_protocols
                .contains_key(self.binding.protocol_id())
        {
            state.ambiguous_protocols.insert(
                self.binding.protocol_id().clone(),
                self.binding.recovery_marker().clone(),
            );
        }
        self.settled.set(true);
        Ok(())
    }
}
