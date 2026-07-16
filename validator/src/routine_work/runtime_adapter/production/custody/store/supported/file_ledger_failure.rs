use super::*;

const MAX_FAILURE_EVIDENCE: usize = 32;

impl FileLedger {
    pub(in crate::routine_work::runtime_adapter::production::custody::store) fn record_failure(
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
        self.transition_payload(
            local,
            PublicationContext::write(&token.grant_id, "failure-evidence", Some(evidence)),
            |payload, _tick, _head| {
                let record = exact_record_mut(payload, token)?;
                if record.state == AttemptState::Ambiguous {
                    let ambiguity = record
                        .publication_ambiguity
                        .as_mut()
                        .ok_or_else(|| error("routine-production-ambiguity-record-missing"))?;
                    match ambiguity.failure_evidence.as_ref() {
                        Some(current) if current == evidence => return Ok(((), false)),
                        Some(_) => {
                            return Err(error(
                                "routine-production-ambiguity-failure-evidence-conflict",
                            ));
                        }
                        None => ambiguity.failure_evidence = Some(evidence.clone()),
                    }
                    return Ok(((), true));
                }
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
            },
        )
    }
}
