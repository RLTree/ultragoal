use super::*;

const MAX_FAILURE_EVIDENCE: usize = 32;

impl FileLedger {
    pub(crate) fn record_failure(
        &self,
        token: &ReservationToken,
        evidence: &ReservationFailureEvidence,
    ) -> Result<(), RoutineError> {
        if token.reuse_only
            || !evidence.shape_is_valid()
            || evidence.protocol_id != token.binding.protocol_id
            || evidence.grant_id != token.grant_id
            || evidence.recovery_marker != token.recovery_marker
        {
            return Err(error("routine-production-failure-evidence-invalid"));
        }
        self.with_payload_conditional(|payload, _tick| {
            let record = exact_record_mut(payload, token)?;
            let state_matches = matches!(
                (record.state, evidence.disposition),
                (
                    AttemptState::Reserved,
                    ReservationFailureDisposition::ReservedPending
                ) | (
                    AttemptState::Started,
                    ReservationFailureDisposition::StartedPending
                )
            );
            if !state_matches {
                return Err(error(
                    "routine-production-failure-evidence-transition-invalid",
                ));
            }
            if record.failure_evidence.last() == Some(evidence) {
                return Ok(((), false));
            }
            if record.failure_evidence.len() >= MAX_FAILURE_EVIDENCE
                || record
                    .failure_evidence
                    .iter()
                    .any(|prior| prior == evidence)
            {
                return Err(error("routine-production-failure-evidence-history-invalid"));
            }
            record.failure_evidence.push(evidence.clone());
            Ok(((), true))
        })
    }

    #[cfg(test)]
    pub(crate) fn test_failure_records(
        &self,
        binding: &AuthorityBinding,
    ) -> Result<Vec<ReservationFailureEvidence>, RoutineError> {
        validate_binding(binding)?;
        self.with_payload(false, |payload, _tick| {
            let record = payload
                .protocols
                .get(&binding.protocol_id)
                .ok_or_else(|| error("routine-production-reservation-missing"))?;
            if record.binding != *binding {
                return Err(error("routine-production-reservation-binding-invalid"));
            }
            Ok(record.failure_evidence.clone())
        })
    }
}
