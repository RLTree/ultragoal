use super::*;

const MAX_FAILURE_EVIDENCE: usize = 32;

impl FileLedger {
    pub(crate) fn record_failure(
        &self,
        local: &mut LocalHead,
        token: &ReservationToken,
        evidence: &ReservationFailureEvidence,
    ) -> Result<DurableWrite<()>, RoutineError> {
        if !evidence.shape_is_valid()
            || evidence.protocol_id != token.binding.protocol_id
            || evidence.grant_id != token.grant_id
            || evidence.recovery_marker != token.recovery_marker
        {
            return Err(error("routine-production-failure-evidence-invalid"));
        }
        self.transition_payload(local, |payload, _tick, _head| {
            let record = exact_record_mut(payload, token)?;
            let state_matches = matches!(
                (record.state, evidence.disposition),
                (
                    AttemptState::Reserved | AttemptState::Staged,
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
}
