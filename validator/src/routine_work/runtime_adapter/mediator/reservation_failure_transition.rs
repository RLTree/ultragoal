use super::read_source_binding::{MediatorRegistry, release_active};
use super::*;

impl AttemptReservation {
    pub(super) fn clear_exact_failure(&self, state: &mut MediatorRegistry) {
        for marker in [
            Some(self.recovery_marker.as_str()),
            self.prior_recovery_marker.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if state.failure_records.get(marker).is_some_and(|record| {
                record.protocol_id == self.protocol_id && record.recovery_marker == marker
            }) {
                state.failure_records.remove(marker);
            }
        }
    }

    pub(super) fn failure_evidence(
        &self,
        primary: FailureEvidence,
        process_cleanup: CleanupEvidence,
        staged_cleanup: CleanupEvidence,
    ) -> ReservationFailureEvidence {
        ReservationFailureEvidence {
            schema_version: RESERVATION_FAILURE_SCHEMA.to_owned(),
            protocol_id: self.protocol_id.clone(),
            grant_id: self.grant_id.clone(),
            recovery_marker: self.recovery_marker.clone(),
            primary,
            process_cleanup,
            staged_cleanup,
            disposition: if self.started.get() {
                ReservationFailureDisposition::StartedPending
            } else {
                ReservationFailureDisposition::ReservedPending
            },
        }
    }

    pub(super) fn record_failure_and_transition(
        &self,
        evidence: &ReservationFailureEvidence,
    ) -> Result<(), RoutineError> {
        if self.settled.get()
            || evidence.protocol_id != self.protocol_id
            || evidence.grant_id != self.grant_id
            || evidence.recovery_marker != self.recovery_marker
            || evidence.disposition
                != if self.started.get() {
                    ReservationFailureDisposition::StartedPending
                } else {
                    ReservationFailureDisposition::ReservedPending
                }
        {
            return Err(mediator_error(
                "mediator-reservation-failure-evidence-binding-invalid",
            ));
        }
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.active_protocols.get(&self.protocol_id) != Some(&self.grant_id) {
            return Err(mediator_error(
                "mediator-reservation-failure-active-binding-invalid",
            ));
        }
        if let Some(durable) = &self.durable {
            durable.record_failure(evidence)?;
        } else {
            match state.failure_records.get(&self.recovery_marker) {
                Some(existing) if existing != evidence => {
                    return Err(mediator_error(
                        "mediator-reservation-failure-evidence-conflict",
                    ));
                }
                Some(_) => {}
                None => {
                    if state.failure_records.len() >= 4_096 {
                        return Err(mediator_error(
                            "mediator-reservation-failure-evidence-capacity-exhausted",
                        ));
                    }
                    state
                        .failure_records
                        .insert(self.recovery_marker.clone(), evidence.clone());
                }
            }
        }
        release_active(&mut state, &self.protocol_id, &self.grant_id);
        if self.started.get() && !state.ambiguous_protocols.contains_key(&self.protocol_id) {
            state
                .ambiguous_protocols
                .insert(self.protocol_id.clone(), self.recovery_marker.clone());
        }
        self.settled.set(true);
        Ok(())
    }
}
