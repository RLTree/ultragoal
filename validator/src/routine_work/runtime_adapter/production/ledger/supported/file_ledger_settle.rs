use super::*;

impl FileLedger {
    pub(crate) fn settle(
        &self,
        token: &ReservationToken,
        state: AttemptState,
        artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        if state.pending()
            || artifacts
                .iter()
                .any(|(digest, witness)| !valid(digest) || !valid(witness))
        {
            return Err(error("routine-production-settlement-invalid"));
        }
        if token.reuse_only {
            if !artifacts.is_empty() {
                return Err(error("routine-production-settlement-invalid"));
            }
            return self.with_payload(false, |payload, _tick| {
                exact_completed_reuse_record(payload, token)?;
                Ok(())
            });
        }
        self.with_payload(true, |payload, _tick| {
            let record = exact_record_mut(payload, token)?;
            let transition_valid = match state {
                AttemptState::Complete if record.reuse_only => {
                    record.state == AttemptState::Reserved && !record.artifacts.is_empty()
                }
                AttemptState::Complete => record.state == AttemptState::Started,
                AttemptState::Failed | AttemptState::Cancelled | AttemptState::Incomplete => {
                    matches!(record.state, AttemptState::Reserved | AttemptState::Started)
                }
                AttemptState::Reserved | AttemptState::Started => false,
            };
            if !transition_valid {
                return Err(error("routine-production-settlement-transition-invalid"));
            }
            if state == AttemptState::Complete && !record.reuse_only {
                if artifacts.is_empty() {
                    return Err(error("routine-production-complete-artifacts-missing"));
                }
                record.artifacts.clone_from(artifacts);
            }
            record.state = state;
            Ok(())
        })
    }
    pub(crate) fn authenticates(
        &self,
        token: &ReservationToken,
        digest: &str,
        witness: &str,
    ) -> Result<bool, RoutineError> {
        if !valid(digest) || !valid(witness) {
            return Ok(false);
        }
        self.with_payload(false, |payload, _tick| {
            let record = if token.reuse_only {
                exact_completed_reuse_record(payload, token)?
            } else {
                exact_record(payload, token)?
            };
            Ok(record
                .artifacts
                .get(digest)
                .is_some_and(|value| value == witness))
        })
    }
    pub(crate) fn pending_recovery(
        &self,
        binding: &AuthorityBinding,
    ) -> Result<Option<PendingRecovery>, RoutineError> {
        validate_binding(binding)?;
        self.with_payload(false, |payload, tick| {
            let Some(record) = payload.protocols.get(&binding.protocol_id) else {
                return Ok(None);
            };
            if record.binding != *binding {
                return Err(error("routine-production-recovery-binding-mismatch"));
            }
            if !record.state.pending() {
                return Ok(None);
            }
            if tick > record.recovery_deadline_tick {
                return Err(error("routine-production-recovery-expired"));
            }
            Ok(Some(PendingRecovery {
                marker: record.recovery_marker.clone(),
                deadline_tick: record.recovery_deadline_tick,
            }))
        })
    }
    #[cfg(test)]
    pub(crate) fn test_expire_pending(
        &self,
        binding: &AuthorityBinding,
    ) -> Result<(), RoutineError> {
        validate_binding(binding)?;
        self.with_payload(true, |payload, tick| {
            let expired = tick
                .checked_sub(1)
                .ok_or_else(|| error("routine-production-test-time-underflow"))?;
            let record = payload
                .protocols
                .get_mut(&binding.protocol_id)
                .ok_or_else(|| error("routine-production-reservation-missing"))?;
            if record.binding != *binding || !record.state.pending() {
                return Err(error("routine-production-test-pending-record-required"));
            }
            record.issued_tick = expired;
            record.expires_tick = expired;
            record.recovery_deadline_tick = expired;
            Ok(())
        })
    }
    #[cfg(test)]
    pub(crate) fn test_expire_reservation(
        &self,
        token: &ReservationToken,
    ) -> Result<u64, RoutineError> {
        self.with_payload(true, |payload, tick| {
            let expired = tick
                .checked_sub(1)
                .ok_or_else(|| error("routine-production-test-time-underflow"))?;
            let record = exact_record_mut(payload, token)?;
            if !matches!(record.state, AttemptState::Reserved | AttemptState::Started) {
                return Err(error("routine-production-test-pending-record-required"));
            }
            record.issued_tick = expired;
            record.expires_tick = expired;
            Ok(expired)
        })
    }
}
