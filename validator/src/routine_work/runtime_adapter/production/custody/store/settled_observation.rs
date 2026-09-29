use super::*;

impl FileLedger {
    pub(in crate::routine_work::runtime_adapter::production::custody::store) fn observe_terminal_settlement(
        &self,
        local: &mut LocalHead,
        binding: &AuthorityBinding,
        attempt_grant: &str,
    ) -> Result<
        crate::routine_work::runtime_adapter::production::RoutineTerminalSettlementProjection,
        RoutineError,
    > {
        validate_binding(binding)?;
        if !valid(attempt_grant) {
            return Err(error("routine-production-settlement-observation-invalid"));
        }
        self.transition_payload(local, PublicationContext::read(), |payload, _tick, head| {
            let record = payload
                .attempts
                .get(attempt_grant)
                .ok_or_else(|| error("routine-production-settlement-attempt-missing"))?;
            if record.binding != *binding || record.grant_id != attempt_grant {
                return Err(error("routine-production-settlement-binding-invalid"));
            }
            let terminal_outcome = match record.state {
                AttemptState::Failed => {
                    crate::routine_work::runtime_adapter::mediator::RoutineTerminalOutcome::Failed
                }
                AttemptState::Cancelled => crate::routine_work::runtime_adapter::mediator::RoutineTerminalOutcome::Cancelled,
                AttemptState::Incomplete => crate::routine_work::runtime_adapter::mediator::RoutineTerminalOutcome::Incomplete,
                AttemptState::Reserved
                | AttemptState::Staged
                | AttemptState::Started
                | AttemptState::Complete
                | AttemptState::RolledBack
                | AttemptState::Ambiguous => {
                    return Err(error("routine-production-settlement-state-unavailable"));
                }
            };
            let terminal = record
                .terminal
                .as_ref()
                .ok_or_else(|| error("routine-production-settlement-record-missing"))?;
            if terminal.state != record.state || record.publication_ambiguity.is_some() {
                return Err(error("routine-production-settlement-record-invalid"));
            }
            Ok((
                crate::routine_work::runtime_adapter::production::RoutineTerminalSettlementProjection::new(
                    super::checkpoint_attestation::continuation_for(record),
                    record.recovery_marker.clone(),
                    record.predecessor_continuations.clone(),
                    record.grant_id.clone(),
                    head.to_owned(),
                    terminal_outcome,
                ),
                false,
            ))
        })?
        .into_result()
    }
}
