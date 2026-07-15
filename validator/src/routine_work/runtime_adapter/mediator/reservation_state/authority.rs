use super::super::grant_validation::reservation_starts_fresh;
use super::binding::ReservationBinding;
use super::durable_binding::DurableBinding;
use super::read_source_binding::release_active;
use super::registry_transition;
use super::*;

#[path = "authority/custody.rs"]
mod custody;
#[path = "authority/staged_custody.rs"]
mod staged_custody;
#[path = "authority/transition_flag.rs"]
mod transition_flag;

use custody::AttemptCustody;

pub(in super::super) struct AttemptReservation {
    binding: ReservationBinding,
    durable: DurableBinding,
    custody: AttemptCustody,
}

pub(in super::super) fn reserve_grant(
    grant: &RoutineRootGrant,
) -> Result<AttemptReservation, RoutineError> {
    if let Some(durable) = &grant.durable {
        durable.validate_reserved()?;
    }
    let mut state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if state.consumed_grants.contains(&grant.grant_id) {
        return Err(mediator_error("mediator-root-grant-replayed"));
    }
    if state.active_protocols.contains_key(&grant.protocol_id) {
        return Err(mediator_error("mediator-protocol-attempt-active"));
    }
    if reservation_starts_fresh(&state, grant)? {
        state
            .failure_records
            .retain(|_, record| record.protocol_id != grant.protocol_id);
    }
    state.consumed_grants.insert(grant.grant_id.clone());
    state
        .active_protocols
        .insert(grant.protocol_id.clone(), grant.grant_id.clone());
    Ok(AttemptReservation {
        binding: ReservationBinding::new(
            grant.protocol_id.clone(),
            grant.grant_id.clone(),
            recovery_identity(&grant.grant_id, &grant.protocol_id, &grant.request_id),
            grant.recovery_for.clone(),
        ),
        durable: DurableBinding::new(grant.durable.clone()),
        custody: AttemptCustody::new(),
    })
}

impl AttemptReservation {
    pub(in super::super) fn protocol_id(&self) -> &String {
        self.binding.protocol_id()
    }

    pub(in super::super) fn grant_id(&self) -> &String {
        self.binding.grant_id()
    }

    pub(in super::super) fn recovery_marker(&self) -> &String {
        self.binding.recovery_marker()
    }

    pub(in super::super) fn is_started(&self) -> bool {
        self.custody.is_started()
    }

    pub(super) fn terminal_is_authoritative(&self) -> bool {
        self.custody.terminal_is_authoritative()
    }

    pub(in super::super) fn reuse_only(&self) -> bool {
        self.durable.reuse_only()
    }

    pub(in super::super) fn prepare_spawn(&self) -> Result<(), RoutineError> {
        self.custody.require_open()?;
        self.durable.prepare_spawn()
    }

    pub(in super::super) fn mark_started(&self) -> Result<(), RoutineError> {
        self.custody.mark_started(&self.binding)
    }

    pub(in super::super) fn stage_success(
        &self,
        artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        self.custody.require_open()?;
        self.custody.require_staged_empty()?;
        self.durable.stage_success(artifacts)
    }

    pub(in super::super) fn settle_success(
        &self,
        artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        self.custody.require_open()?;
        self.custody.require_staged_empty()?;
        self.durable
            .settle(DurableSettlement::Complete, artifacts)?;
        self.custody.finish_terminal(&self.binding, true);
        Ok(())
    }

    pub(in super::super) fn settle_incomplete(
        &self,
        outcome: DurableSettlement,
    ) -> Result<Option<String>, RoutineError> {
        self.custody.require_open()?;
        self.custody.require_staged_empty()?;
        let durable = self.durable.settle(outcome, &BTreeMap::new())?;
        let pending = self.custody.finish_terminal(&self.binding, durable);
        Ok((!durable).then_some(pending).flatten())
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
        self.custody.require_open()?;
        self.custody.require_staged_empty()?;
        let staged = self.durable.stage_program(program)?;
        self.custody.require_open()?;
        self.custody.require_staged_empty()?;
        self.custody.stage_and_use(staged, use_program)
    }

    pub(super) fn cleanup_staged(&self) -> Result<(), RoutineError> {
        self.custody
            .cleanup_staged(self.durable.is_present(), |staged| {
                self.durable.cleanup_staged(staged)
            })
    }

    pub(super) fn failure_evidence(
        &self,
        primary: FailureEvidence,
        process_cleanup: CleanupEvidence,
        staged_cleanup: CleanupEvidence,
    ) -> ReservationFailureEvidence {
        self.binding.failure_evidence(
            primary,
            process_cleanup,
            staged_cleanup,
            self.custody.is_started(),
        )
    }

    pub(super) fn record_failure_and_transition(
        &self,
        evidence: &ReservationFailureEvidence,
    ) -> Result<(), RoutineError> {
        self.custody.require_open()?;
        if !self
            .binding
            .validates_failure(evidence, self.custody.is_started())
        {
            return Err(mediator_error(
                "mediator-reservation-failure-evidence-binding-invalid",
            ));
        }
        let transfer = self
            .custody
            .failure_transfer_required(&evidence.staged_cleanup, self.durable.is_present())?;
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.active_protocols.get(self.protocol_id()) != Some(self.grant_id()) {
            return Err(mediator_error(
                "mediator-reservation-failure-active-binding-invalid",
            ));
        }
        if !self.durable.record_failure(evidence)? {
            registry_transition::record_non_durable_failure(&mut state, &self.binding, evidence)?;
        }
        self.custody.finish_failure(transfer, |started| {
            release_active(&mut state, self.protocol_id(), self.grant_id());
            if started && !state.ambiguous_protocols.contains_key(self.protocol_id()) {
                state
                    .ambiguous_protocols
                    .insert(self.protocol_id().clone(), self.recovery_marker().clone());
            }
        });
        Ok(())
    }
}
